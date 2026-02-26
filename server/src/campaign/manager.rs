use crate::campaign::CampaignManager;
use crate::database::models::{Campaign, CampaignStatus, Session, SessionStatus};
use crate::database::repository::{CampaignRepository, SessionRepository, AuditRepository};
use crate::logging::AuditLogger;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use uuid::Uuid;

/// Manages the lifecycle and state of campaigns
pub struct CampaignManager {
    repository: Arc<CampaignRepository<'static>>,
    session_repo: Arc<SessionRepository<'static>>,
    audit: Arc<AuditLogger>,
    /// In-memory cache of active campaigns
    active_campaigns: RwLock<HashMap<String, CampaignState>>,
}

#[derive(Debug, Clone)]
struct CampaignState {
    campaign: Campaign,
    sessions: Vec<Session>,
    operator_ids: Vec<String>,
}

impl CampaignManager {
    pub fn new(
        repository: Arc<CampaignRepository<'static>>,
        session_repo: Arc<SessionRepository<'static>>,
        audit: Arc<AuditLogger>,
    ) -> Self {
        Self {
            repository,
            session_repo,
            audit,
            active_campaigns: RwLock::new(HashMap::new()),
        }
    }
    
    /// Initialize by loading active campaigns from database
    pub async fn initialize(&self) -> crate::Result<()> {
        let campaigns = self.repository.list_active().await?;
        
        let mut cache = self.active_campaigns.write().await;
        for campaign in campaigns {
            let sessions = self.session_repo.list_by_campaign(&campaign.id).await?;
            let state = CampaignState {
                operator_ids: vec![campaign.operator_id.clone()],
                campaign,
                sessions,
            };
            cache.insert(state.campaign.id.clone(), state);
        }
        
        tracing::info!("Loaded {} active campaigns", cache.len());
        Ok(())
    }
    
    /// Create a new campaign
    pub async fn create_campaign(
        &self,
        name: String,
        description: Option<String>,
        operator_id: String,
        metadata: Option<serde_json::Value>,
    ) -> crate::Result<Campaign> {
        let campaign = Campaign {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            operator_id: operator_id.clone(),
            status: CampaignStatus::Planning,
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
            metadata: metadata.map(|m| m.to_string()),
        };
        
        self.repository.create(&campaign).await?;
        
        // Add to cache
        let mut cache = self.active_campaigns.write().await;
        cache.insert(campaign.id.clone(), CampaignState {
            campaign: campaign.clone(),
            sessions: vec![],
            operator_ids: vec![operator_id],
        });
        
        // Audit log
        self.audit.log(
            &operator_id,
            "campaign_create",
            None,
            Some(&campaign.id),
            Some(&format!("Created campaign: {}", campaign.name)),
            None,
            true,
        ).await?;
        
        tracing::info!("Campaign {} created by {}", campaign.id, operator_id);
        Ok(campaign)
    }
    
    /// Start a campaign (transition from Planning to Active)
    pub async fn start_campaign(
        &self,
        campaign_id: &str,
        operator_id: &str,
    ) -> crate::Result<()> {
        // Verify operator owns this campaign
        {
            let cache = self.active_campaigns.read().await;
            let state = cache.get(campaign_id)
                .ok_or_else(|| crate::OblivionError::Protocol("Campaign not found".into()))?;
            
            if state.campaign.operator_id != operator_id && !state.operator_ids.contains(&operator_id.to_string()) {
                return Err(crate::OblivionError::AuthenticationFailed);
            }
        }
        
        self.repository.update_status(campaign_id, CampaignStatus::Active).await?;
        
        // Update cache
        let mut cache = self.active_campaigns.write().await;
        if let Some(state) = cache.get_mut(campaign_id) {
            state.campaign.status = CampaignStatus::Active;
            state.campaign.started_at = Some(Utc::now());
        }
        
        self.audit.log(
            operator_id,
            "campaign_start",
            None,
            Some(campaign_id),
            None,
            None,
            true,
        ).await?;
        
        tracing::info!("Campaign {} started by {}", campaign_id, operator_id);
        Ok(())
    }
    
    /// Pause a campaign (temporarily halt new sessions)
    pub async fn pause_campaign(
        &self,
        campaign_id: &str,
        operator_id: &str,
    ) -> crate::Result<()> {
        self.verify_operator_access(campaign_id, operator_id).await?;
        
        self.repository.update_status(campaign_id, CampaignStatus::Paused).await?;
        
        let mut cache = self.active_campaigns.write().await;
        if let Some(state) = cache.get_mut(campaign_id) {
            state.campaign.status = CampaignStatus::Paused;
        }
        
        self.audit.log(
            operator_id,
            "campaign_pause",
            None,
            Some(campaign_id),
            None,
            None,
            true,
        ).await?;
        
        Ok(())
    }
    
    /// Close a campaign (end operations, keep data)
    pub async fn close_campaign(
        &self,
        campaign_id: &str,
        operator_id: &str,
    ) -> crate::Result<()> {
        self.verify_operator_access(campaign_id, operator_id).await?;
        
        // Terminate all active sessions first
        let sessions = self.session_repo.list_by_campaign(campaign_id).await?;
        for session in sessions {
            if matches!(session.status, SessionStatus::Active | SessionStatus::Idle) {
                // Signal session termination (actual termination handled by session manager)
                self.session_repo.update_status(&session.id, SessionStatus::Terminated).await?;
            }
        }
        
        self.repository.update_status(campaign_id, CampaignStatus::Closing).await?;
        
        let mut cache = self.active_campaigns.write().await;
        if let Some(state) = cache.get_mut(campaign_id) {
            state.campaign.status = CampaignStatus::Closing;
            state.campaign.ended_at = Some(Utc::now());
        }
        
        // Remove from active cache after a delay (or keep for monitoring close process)
        cache.remove(campaign_id);
        
        self.audit.log(
            operator_id,
            "campaign_close",
            None,
            Some(campaign_id),
            None,
            None,
            true,
        ).await?;
        
        tracing::info!("Campaign {} closed by {}", campaign_id, operator_id);
        Ok(())
    }
    
    /// Archive a campaign (read-only, long-term storage)
    pub async fn archive_campaign(
        &self,
        campaign_id: &str,
        operator_id: &str,
    ) -> crate::Result<()> {
        self.verify_operator_access(campaign_id, operator_id).await?;
        
        self.repository.update_status(campaign_id, CampaignStatus::Archived).await?;
        
        self.audit.log(
            operator_id,
            "campaign_archive",
            None,
            Some(campaign_id),
            None,
            None,
            true,
        ).await?;
        
        Ok(())
    }
    
    /// Check if a campaign can accept new sessions
    pub async fn can_accept_sessions(&self, campaign_id: &str) -> bool {
        let cache = self.active_campaigns.read().await;
        if let Some(state) = cache.get(campaign_id) {
            matches!(state.campaign.status, CampaignStatus::Active)
        } else {
            false
        }
    }
    
    /// Get campaign details with current sessions
    pub async fn get_campaign_details(
        &self,
        campaign_id: &str,
    ) -> crate::Result<(Campaign, Vec<Session>)> {
        let cache = self.active_campaigns.read().await;
        if let Some(state) = cache.get(campaign_id) {
            Ok((state.campaign.clone(), state.sessions.clone()))
        } else {
            // Fallback to database
            let campaign = self.repository.get_by_id(campaign_id).await?
                .ok_or_else(|| crate::OblivionError::Protocol("Campaign not found".into()))?;
            let sessions = self.session_repo.list_by_campaign(campaign_id).await?;
            Ok((campaign, sessions))
        }
    }
    
    /// List all active campaigns for an operator
    pub async fn list_operator_campaigns(
        &self,
        operator_id: &str,
    ) -> Vec<Campaign> {
        let cache = self.active_campaigns.read().await;
        cache.values()
            .filter(|state| state.operator_ids.contains(&operator_id.to_string()))
            .map(|state| state.campaign.clone())
            .collect()
    }
    
    /// Add operator to campaign (multiplayer support)
    pub async fn add_operator(
        &self,
        campaign_id: &str,
        requesting_operator: &str,
        new_operator: String,
    ) -> crate::Result<()> {
        self.verify_operator_access(campaign_id, requesting_operator).await?;
        
        let mut cache = self.active_campaigns.write().await;
        if let Some(state) = cache.get_mut(campaign_id) {
            if !state.operator_ids.contains(&new_operator) {
                state.operator_ids.push(new_operator.clone());
            }
        }
        
        self.audit.log(
            requesting_operator,
            "campaign_add_operator",
            None,
            Some(campaign_id),
            Some(&format!("Added operator: {}", new_operator)),
            None,
            true,
        ).await?;
        
        Ok(())
    }
    
    /// Update session list in cache (called by session manager)
    pub async fn update_campaign_sessions(
        &self,
        campaign_id: &str,
        sessions: Vec<Session>,
    ) -> crate::Result<()> {
        let mut cache = self.active_campaigns.write().await;
        if let Some(state) = cache.get_mut(campaign_id) {
            state.sessions = sessions;
        }
        Ok(())
    }
    
    /// Verify operator has access to campaign
    async fn verify_operator_access(
        &self,
        campaign_id: &str,
        operator_id: &str,
    ) -> crate::Result<()> {
        let cache = self.active_campaigns.read().await;
        let state = cache.get(campaign_id)
            .ok_or_else(|| crate::OblivionError::Protocol("Campaign not found".into()))?;
        
        if state.campaign.operator_id != operator_id && !state.operator_ids.contains(&operator_id.to_string()) {
            return Err(crate::OblivionError::AuthenticationFailed);
        }
        
        Ok(())
    }
    
    /// Get campaign statistics
    pub async fn get_statistics(&self, campaign_id: &str) -> crate::Result<CampaignStatistics> {
        let (_, sessions) = self.get_campaign_details(campaign_id).await?;
        
        let total_sessions = sessions.len();
        let active_sessions = sessions.iter()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .count();
        let stale_sessions = sessions.iter()
            .filter(|s| matches!(s.status, SessionStatus::Stale))
            .count();
        
        // Calculate duration
        let cache = self.active_campaigns.read().await;
        let duration_secs = cache.get(campaign_id)
            .and_then(|s| s.campaign.started_at)
            .map(|start| (Utc::now() - start).num_seconds())
            .unwrap_or(0);
        
        Ok(CampaignStatistics {
            total_sessions,
            active_sessions,
            stale_sessions,
            terminated_sessions: total_sessions - active_sessions - stale_sessions,
            duration_secs: duration_secs as u64,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CampaignStatistics {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub stale_sessions: usize,
    pub terminated_sessions: usize,
    pub duration_secs: u64,
}

impl CampaignManager {
    /// Auto-archive old campaigns (background task)
    pub async fn auto_archive_old_campaigns(&self, days: i64) -> crate::Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        
        let campaigns = self.repository.list_active().await?;
        let mut archived = 0;
        
        for campaign in campaigns {
            if let Some(ended) = campaign.ended_at {
                if ended < cutoff && matches!(campaign.status, CampaignStatus::Closing) {
                    self.repository.update_status(&campaign.id, CampaignStatus::Archived).await?;
                    archived += 1;
                    
                    tracing::info!("Auto-archived campaign {} (ended {})", campaign.id, ended);
                }
            }
        }
        
        Ok(archived)
    }
}
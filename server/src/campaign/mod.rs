pub mod manager;

pub use manager::{CampaignManager, CampaignStatistics};

use crate::database::models::CampaignStatus;

impl CampaignManager {
    pub fn can_accept_sessions(&self, status: CampaignStatus) -> bool {
        matches!(status, CampaignStatus::Active)
    }
    
    pub fn is_archived(&self, status: CampaignStatus) -> bool {
        matches!(status, CampaignStatus::Archived)
    }
}
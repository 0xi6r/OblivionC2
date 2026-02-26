import { create } from "zustand";
import { invoke } from "@tauri-apps/api/tauri";

export interface Campaign {
  id: string;
  name: string;
  description: string;
  operator_id: string;
  status: number;
  created_at: string;
  started_at?: string;
  ended_at?: string;
}

export interface CampaignDetails {
  campaign: Campaign;
  sessions: Session[];
  statistics: CampaignStatistics;
}

export interface CampaignStatistics {
  total_sessions: number;
  active_sessions: number;
  stale_sessions: number;
  terminated_sessions: number;
  duration_seconds: number;
}

export interface Session {
  id: string;
  campaign_id: string;
  hostname: string;
  username: string;
  os_version: string;
  process_id: number;
  first_seen: string;
  last_seen: string;
  status: number;
}

interface CampaignState {
  campaigns: Campaign[];
  currentCampaign: CampaignDetails | null;
  isLoading: boolean;
  
  loadCampaigns: () => Promise<void>;
  createCampaign: (name: string, description?: string) => Promise<void>;
  selectCampaign: (id: string) => Promise<void>;
  controlCampaign: (id: string, action: "start" | "pause" | "close" | "archive") => Promise<void>;
  refreshSessions: () => Promise<void>;
}

export const useCampaignStore = create<CampaignState>((set, get) => ({
  campaigns: [],
  currentCampaign: null,
  isLoading: false,

  loadCampaigns: async () => {
    set({ isLoading: true });
    try {
      const campaigns = await invoke<Campaign[]>("list_campaigns");
      set({ campaigns, isLoading: false });
    } catch (err) {
      console.error("Failed to load campaigns:", err);
      set({ isLoading: false });
    }
  },

  createCampaign: async (name, description) => {
    try {
      const campaign = await invoke<Campaign>("create_campaign", { name, description });
      set((state) => ({ campaigns: [...state.campaigns, campaign] }));
    } catch (err) {
      console.error("Failed to create campaign:", err);
      throw err;
    }
  },

  selectCampaign: async (id) => {
    set({ isLoading: true });
    try {
      const details = await invoke<CampaignDetails>("get_campaign_details", {
        campaignId: id,
      });
      set({ currentCampaign: details, isLoading: false });
    } catch (err) {
      console.error("Failed to load campaign details:", err);
      set({ isLoading: false });
    }
  },

  controlCampaign: async (id, action) => {
    try {
      await invoke("control_campaign", { campaignId: id, action });
      await get().loadCampaigns();
      if (get().currentCampaign?.campaign.id === id) {
        await get().selectCampaign(id);
      }
    } catch (err) {
      console.error("Failed to control campaign:", err);
      throw err;
    }
  },

  refreshSessions: async () => {
    const current = get().currentCampaign;
    if (!current) return;
    
    try {
      const sessions = await invoke<Session[]>("list_sessions", {
        campaignId: current.campaign.id,
      });
      set((state) => ({
        currentCampaign: state.currentCampaign
          ? { ...state.currentCampaign, sessions }
          : null,
      }));
    } catch (err) {
      console.error("Failed to refresh sessions:", err);
    }
  },
}));
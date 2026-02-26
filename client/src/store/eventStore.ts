import { create } from "zustand";
import { listen, Event as TauriEvent } from "@tauri-apps/api/event";
import { useCampaignStore } from "./campaignStore";

export interface C2Event {
  event_type: string;
  data: any;
  timestamp: string;
}

interface EventState {
  isSubscribed: boolean;
  events: C2Event[];
  currentCampaignId: string | null;
  
  startEventStream: (campaignId: string) => Promise<void>;
  stopEventStream: () => void;
  handleEvent: (event: C2Event) => void;
  clearEvents: () => void;
}

export const useEventStore = create<EventState>((set, get) => ({
  isSubscribed: false,
  events: [],
  currentCampaignId: null,

  startEventStream: async (campaignId) => {
    if (get().isSubscribed) {
      get().stopEventStream();
    }
    
    try {
      // Subscribe via Tauri command
      const { invoke } = await import("@tauri-apps/api/tauri");
      await invoke("subscribe_events", { campaignId });
      
      // Listen for events from Rust
      const unlisten = await listen("c2-event", (event: TauriEvent<C2Event>) => {
        get().handleEvent(event.payload);
      });
      
      set({ 
        isSubscribed: true, 
        currentCampaignId: campaignId,
        events: [] 
      });
      
      // Store unlisten for cleanup
      (window as any).__eventUnlisten = unlisten;
    } catch (error) {
      console.error("Failed to start event stream:", error);
      throw error;
    }
  },

  stopEventStream: () => {
    const unlisten = (window as any).__eventUnlisten;
    if (unlisten) {
      unlisten();
    }
    
    set({ 
      isSubscribed: false, 
      currentCampaignId: null 
    });
  },

  handleEvent: (event) => {
    // Add to event log
    set((state) => ({
      events: [event, ...state.events].slice(0, 1000), // Keep last 1000
    }));
    
    // Handle specific event types
    switch (event.event_type) {
      case "CAMPAIGN_UPDATE":
        // Refresh campaign data
        useCampaignStore.getState().loadCampaigns();
        break;
        
      case "SESSION_CONNECT":
      case "SESSION_DISCONNECT":
        // Refresh sessions if viewing this campaign
        const currentCampaign = useCampaignStore.getState().currentCampaign;
        if (currentCampaign && event.data.campaign?.id === currentCampaign.campaign.id) {
          useCampaignStore.getState().refreshSessions();
        }
        break;
        
      case "TASK_COMPLETE":
        // Could trigger notification or auto-refresh
        break;
        
      default:
        console.log("Unknown event type:", event.event_type);
    }
  },

  clearEvents: () => {
    set({ events: [] });
  },
}));
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/tauri";

interface ConnectionState {
  isConnected: boolean;
  serverAddress: string;
  operatorId: string;
  isLoading: boolean;
  error: string | null;
  
  connect: (address: string, operatorId: string, cert?: string) => Promise<void>;
  disconnect: () => Promise<void>;
  checkConnection: () => Promise<void>;
}

export const useConnectionStore = create<ConnectionState>((set) => ({
  isConnected: false,
  serverAddress: "",
  operatorId: "",
  isLoading: false,
  error: null,

  connect: async (address, operatorId, cert) => {
    set({ isLoading: true, error: null });
    try {
      const success = await invoke("connect_to_server", {
        address,
        operatorId,
        clientCert: cert,
        clientKey: null,
        caCert: null,
      });
      
      if (success) {
        set({ 
          isConnected: true, 
          serverAddress: address, 
          operatorId,
          isLoading: false 
        });
      } else {
        set({ error: "Connection failed", isLoading: false });
      }
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  disconnect: async () => {
    await invoke("disconnect");
    set({ isConnected: false });
  },

  checkConnection: async () => {
    const status = await invoke("get_connection_status");
    set({ isConnected: status as boolean });
  },
}));
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/tauri";

export interface Task {
  id: number;
  sessionId: string;
  campaignId: string;
  taskType: string;
  status: "pending" | "in_progress" | "completed" | "failed" | "timeout";
  output?: string;
  error?: string;
  createdAt: Date;
  completedAt?: Date;
}

interface TaskState {
  tasks: Task[];
  activeTask: Task | null;
  isExecuting: boolean;
  
  executeTask: (campaignId: string, sessionId: string, taskType: string, payload: string) => Promise<void>;
  executeBroadcast: (campaignId: string, sessionIds: string[], taskType: string, payload: string) => Promise<void>;
  pollForResult: (taskId: number) => Promise<void>;
  getTaskHistory: (sessionId: string) => Task[];
  clearCompleted: () => void;
}

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  activeTask: null,
  isExecuting: false,

  executeTask: async (campaignId, sessionId, taskType, payload) => {
    set({ isExecuting: true });
    
    try {
      const taskId = await invoke<number>("create_task", {
        campaignId,
        sessionId,
        taskType,
        payload: new TextEncoder().encode(payload),
        timeout: 60,
      });
      
      const newTask: Task = {
        id: taskId,
        sessionId,
        campaignId,
        taskType,
        status: "pending",
        createdAt: new Date(),
      };
      
      set((state) => ({
        tasks: [...state.tasks, newTask],
        activeTask: newTask,
      }));
      
      // Start polling
      await get().pollForResult(taskId);
    } catch (error) {
      console.error("Task execution failed:", error);
      throw error;
    } finally {
      set({ isExecuting: false });
    }
  },

  executeBroadcast: async (campaignId, sessionIds, taskType, payload) => {
    set({ isExecuting: true });
    
    try {
      const taskIds = await invoke<number[]>("create_broadcast_task", {
        campaignId,
        sessionIds,
        taskType,
        payload: new TextEncoder().encode(payload),
        timeout: 60,
      });
      
      // Create task entries for all
      const newTasks: Task[] = taskIds.map((id, idx) => ({
        id,
        sessionId: sessionIds[idx],
        campaignId,
        taskType,
        status: "pending",
        createdAt: new Date(),
      }));
      
      set((state) => ({
        tasks: [...state.tasks, ...newTasks],
      }));
      
      // Poll all tasks
      await Promise.all(taskIds.map((id) => get().pollForResult(id)));
    } catch (error) {
      console.error("Broadcast execution failed:", error);
      throw error;
    } finally {
      set({ isExecuting: false });
    }
  },

  pollForResult: async (taskId) => {
    const maxAttempts = 60;
    const delayMs = 1000;
    
    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      try {
        const result = await invoke<any>("get_task_result", { taskId });
        
        if (result.status === 2) { // Completed
          const output = result.output 
            ? new TextDecoder().decode(new Uint8Array(result.output))
            : "";
          
          set((state) => ({
            tasks: state.tasks.map((t) =>
              t.id === taskId
                ? { ...t, status: "completed", output, completedAt: new Date() }
                : t
            ),
            activeTask: null,
          }));
          return;
        } else if (result.status === 3) { // Failed
          set((state) => ({
            tasks: state.tasks.map((t) =>
              t.id === taskId
                ? { ...t, status: "failed", error: result.error, completedAt: new Date() }
                : t
            ),
            activeTask: null,
          }));
          return;
        } else if (result.status === 4) { // Timeout
          set((state) => ({
            tasks: state.tasks.map((t) =>
              t.id === taskId
                ? { ...t, status: "timeout", completedAt: new Date() }
                : t
            ),
            activeTask: null,
          }));
          return;
        }
        
        // Still pending, wait and retry
        await new Promise((resolve) => setTimeout(resolve, delayMs));
      } catch (error) {
        console.error(`Poll attempt ${attempt} failed:`, error);
      }
    }
    
    // Max attempts reached
    set((state) => ({
      tasks: state.tasks.map((t) =>
        t.id === taskId ? { ...t, status: "timeout" } : t
      ),
      activeTask: null,
    }));
  },

  getTaskHistory: (sessionId) => {
    return get().tasks.filter((t) => t.sessionId === sessionId);
  },

  clearCompleted: () => {
    set((state) => ({
      tasks: state.tasks.filter((t) => t.status === "pending" || t.status === "in_progress"),
    }));
  },
}));
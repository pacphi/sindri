import { create } from "zustand";

interface UIState {
  sidebarCollapsed: boolean;
  activeInstanceId: string | null;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setActiveInstance: (id: string | null) => void;
}

export const useUIStore = create<UIState>((set) => ({
  sidebarCollapsed: false,
  activeInstanceId: null,

  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

  setSidebarCollapsed: (collapsed: boolean) => set({ sidebarCollapsed: collapsed }),

  setActiveInstance: (id: string | null) => set({ activeInstanceId: id }),
}));

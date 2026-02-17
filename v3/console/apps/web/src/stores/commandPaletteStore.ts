import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type PaletteMode = 'command' | 'instance' | 'search' | 'shortcuts'

interface CommandPaletteState {
  isOpen: boolean
  mode: PaletteMode
  recentInstanceIds: string[]
  openPalette: (mode?: PaletteMode) => void
  closePalette: () => void
  togglePalette: (mode?: PaletteMode) => void
  addRecentInstance: (id: string) => void
}

export const useCommandPaletteStore = create<CommandPaletteState>()(
  persist(
    (set) => ({
      isOpen: false,
      mode: 'command',
      recentInstanceIds: [],

      openPalette: (mode = 'command') => set({ isOpen: true, mode }),
      closePalette: () => set({ isOpen: false }),
      togglePalette: (mode = 'command') =>
        set((state) => ({
          isOpen: !state.isOpen,
          mode: state.isOpen ? state.mode : mode,
        })),
      addRecentInstance: (id) =>
        set((state) => ({
          recentInstanceIds: [id, ...state.recentInstanceIds.filter((rid) => rid !== id)].slice(0, 10),
        })),
    }),
    {
      name: 'command-palette',
      partialize: (state) => ({ recentInstanceIds: state.recentInstanceIds }),
    },
  ),
)

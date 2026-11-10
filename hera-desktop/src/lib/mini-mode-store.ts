import { create } from 'zustand';

import { type MiniOverlayLayoutMode, setMiniOverlayMode } from './bridge-hera';

type MiniOverlayTab = 'siguientes' | 'puente';

type MiniModeState = {
  layoutMode: MiniOverlayLayoutMode;
  pinned: boolean;
  activeTab: MiniOverlayTab;
  enterMiniCard: () => Promise<void>;
  collapseToPill: () => Promise<void>;
  expandToCard: () => Promise<void>;
  restoreFullWindow: () => Promise<void>;
  toggleMiniMode: () => Promise<void>;
  togglePinned: () => Promise<void>;
  setActiveTab: (tab: MiniOverlayTab) => void;
};

export const useMiniModeStore = create<MiniModeState>((set, get) => ({
  layoutMode: 'full',
  pinned: true,
  activeTab: 'siguientes',

  enterMiniCard: async () => {
    const { pinned } = get();
    set({ layoutMode: 'card' });
    await setMiniOverlayMode('card', pinned);
  },

  collapseToPill: async () => {
    const { pinned } = get();
    set({ layoutMode: 'pill' });
    await setMiniOverlayMode('pill', pinned);
  },

  expandToCard: async () => {
    const { pinned } = get();
    set({ layoutMode: 'card' });
    await setMiniOverlayMode('card', pinned);
  },

  restoreFullWindow: async () => {
    set({ layoutMode: 'full' });
    await setMiniOverlayMode('full', false);
  },

  toggleMiniMode: async () => {
    const { layoutMode, pinned } = get();
    if (layoutMode === 'full') {
      set({ layoutMode: 'card' });
      await setMiniOverlayMode('card', pinned);
    } else {
      set({ layoutMode: 'full' });
      await setMiniOverlayMode('full', false);
    }
  },

  togglePinned: async () => {
    const { layoutMode, pinned } = get();
    const nextPinned = !pinned;
    set({ pinned: nextPinned });
    if (layoutMode !== 'full') {
      await setMiniOverlayMode(layoutMode, nextPinned);
    }
  },

  setActiveTab: (tab) => {
    set({ activeTab: tab });
  },
}));

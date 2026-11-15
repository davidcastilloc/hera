import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';

export interface ScannerRowPatch {
  track_id: string;
  bpm?: number | null;
  camelot?: string | null;
  energy?: number | null;
  scan_level?: number | null;
  is_transition_candidate?: boolean | null;
  is_fake_320kbps?: boolean | null;
}

export interface ScannerTelemetry {
  fastDone: number;
  fullDone: number;
  total: number;
  tracksPerSec: number;
  etaSec: number;
  governorActive: boolean;
}

interface ScannerStoreState extends ScannerTelemetry {
  isPointerInsideTable: boolean;
  pendingFrozenPatches: ScannerRowPatch[];
  setTelemetry: (t: Partial<ScannerTelemetry>) => void;
  setPointerInsideTable: (inside: boolean) => void;
  queueFrozenPatches: (patches: ScannerRowPatch[]) => void;
  clearFrozenPatches: () => ScannerRowPatch[];
}

export const useScannerStore = create<ScannerStoreState>((set, get) => ({
  fastDone: 0,
  fullDone: 0,
  total: 0,
  tracksPerSec: 0,
  etaSec: 0,
  governorActive: false,
  isPointerInsideTable: false,
  pendingFrozenPatches: [],
  setTelemetry: (t) => set((state) => ({ ...state, ...t })),
  setPointerInsideTable: (inside) => set({ isPointerInsideTable: inside }),
  queueFrozenPatches: (patches) =>
    set((state) => ({
      pendingFrozenPatches: [...state.pendingFrozenPatches, ...patches],
    })),
  clearFrozenPatches: () => {
    const pending = get().pendingFrozenPatches;
    set({ pendingFrozenPatches: [] });
    return pending;
  },
}));

let hoverTimer: ReturnType<typeof setTimeout> | null = null;
let viewportTimer: ReturnType<typeof setTimeout> | null = null;

/** Hover Pre-Warming (300 ms): promueve el track bajo el cursor a P0Immediate. */
export function scheduleHoverPrewarm(trackId: string): void {
  if (hoverTimer) clearTimeout(hoverTimer);
  hoverTimer = setTimeout(() => {
    void invoke<number>('plugin:hera-dj|scanner_prewarm_track', { trackId }).catch(() => {});
  }, 300);
}

export function cancelHoverPrewarm(): void {
  if (hoverTimer) {
    clearTimeout(hoverTimer);
    hoverTimer = null;
  }
}

/** Viewport Lookahead (120 ms): promueve los tracks visibles a P1Viewport. */
export function scheduleViewportBoost(trackIds: string[]): void {
  if (viewportTimer) clearTimeout(viewportTimer);
  viewportTimer = setTimeout(() => {
    if (trackIds.length > 0) {
      void invoke<number>('plugin:hera-dj|scanner_boost_viewport', { trackIds }).catch(() => {});
    }
  }, 120);
}

export function notifyDeckPlaying(playing: boolean): void {
  void invoke<number>('plugin:hera-dj|scanner_set_deck_playing', { playing }).catch(() => {});
}

export function notifyDeckTrack(trackId: string, camelotNeighbors: string[]): void {
  void invoke<number>('plugin:hera-dj|scanner_notify_deck_track', {
    trackId,
    camelotNeighbors,
  }).catch(() => {});
}

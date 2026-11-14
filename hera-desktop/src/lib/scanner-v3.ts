import { invoke } from '@tauri-apps/api/core';

let hoverTimer: ReturnType<typeof setTimeout> | null = null;
let viewportTimer: ReturnType<typeof setTimeout> | null = null;

/** Hover Pre-Warming (300 ms): promueve el track bajo el cursor a P0Immediate. */
export function scheduleHoverPrewarm(trackId: string): void {
  if (hoverTimer) clearTimeout(hoverTimer);
  hoverTimer = setTimeout(() => {
    void invoke<number>('scanner_prewarm_track', { trackId }).catch(() => {});
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
      void invoke<number>('scanner_boost_viewport', { trackIds }).catch(() => {});
    }
  }, 120);
}

export function notifyDeckPlaying(playing: boolean): void {
  void invoke<number>('scanner_set_deck_playing', { playing }).catch(() => {});
}

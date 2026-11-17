import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import type {
  Config,
  IPCEvent,
  ScanProgress,
  SortBy,
  Track,
} from '../generated/typings';
import { useMiniModeStore } from '../lib/mini-mode-store';
import type { HydratedFolderCrate } from '../lib/omkf-bridge';
import player from '../lib/player';
import { configQuery } from '../lib/queries';
import queryClient from '../lib/query-client';
import {
  type ScannerRowPatch,
  type ScannerTelemetry,
  markTracksRecentlyUpdated,
  useScannerStore,
} from '../lib/scanner-v3';
import useLibraryStore from '../lib/store';

export interface OmkfFolderUpdatedPayload {
  crate_ids?: string[];
}

/**
 * Deduplicated O(N + M) merge by `id`. Existing tracks are updated in-place,
 * newly indexed tracks are appended.
 */
export function mergeIncomingTrackBatch(
  existing: Track[],
  incoming: Track[],
): Track[] {
  if (!incoming || incoming.length === 0) return existing;
  if (!existing || existing.length === 0) return [...incoming];

  const existingMap = new Map<string, number>();
  const result: Track[] = existing.map((t, idx) => {
    existingMap.set(t.id, idx);
    return t;
  });

  for (const track of incoming) {
    const idx = existingMap.get(track.id);
    if (idx !== undefined) {
      result[idx] = { ...result[idx], ...track };
    } else {
      existingMap.set(track.id, result.length);
      result.push(track);
    }
  }

  return result;
}

/**
 * Remove track IDs from an existing track array.
 */
export function removeTrackIds(
  existing: Track[],
  removedIds: string[],
): Track[] {
  if (
    !removedIds ||
    removedIds.length === 0 ||
    !existing ||
    existing.length === 0
  ) {
    return existing;
  }
  const removeSet = new Set(removedIds);
  return existing.filter((t) => !removeSet.has(t.id));
}

/**
 * Apply scanner row patches to tracks.
 */
export function applyRowPatchesToTracks(
  existing: Track[],
  patches: ScannerRowPatch[],
): Track[] {
  if (!patches || patches.length === 0 || !existing || existing.length === 0) {
    return existing;
  }
  const patchMap = new Map<string, ScannerRowPatch>();
  for (const p of patches) {
    patchMap.set(p.track_id, p);
  }

  let changed = false;
  const result = existing.map((track) => {
    const p = patchMap.get(track.id);
    if (!p) return track;
    changed = true;
    const patched: Track = {
      ...track,
      bpm: p.bpm != null ? p.bpm : track.bpm,
      camelot: p.camelot != null ? p.camelot : track.camelot,
      energy: p.energy != null ? p.energy : track.energy,
      duration: p.duration != null ? p.duration : track.duration,
      hot_cues: p.hot_cues != null ? p.hot_cues : track.hot_cues,
      scan_level: p.scan_level != null ? p.scan_level : track.scan_level,
      is_transition_candidate:
        p.is_transition_candidate != null
          ? p.is_transition_candidate
          : track.is_transition_candidate,
      is_fake_320kbps:
        p.is_fake_320kbps != null ? p.is_fake_320kbps : track.is_fake_320kbps,
      waveform_peaks_b64:
        p.waveform_peaks_b64 != null
          ? p.waveform_peaks_b64
          : track.waveform_peaks_b64,
      has_audio_skips:
        p.has_audio_skips != null ? p.has_audio_skips : track.has_audio_skips,
      is_variable_bpm:
        p.is_variable_bpm != null ? p.is_variable_bpm : track.is_variable_bpm,
      has_intro_vocals:
        p.has_intro_vocals != null
          ? p.has_intro_vocals
          : track.has_intro_vocals,
      has_outro_vocals:
        p.has_outro_vocals != null
          ? p.has_outro_vocals
          : track.has_outro_vocals,
      replay_gain_db:
        p.replay_gain_db != null ? p.replay_gain_db : track.replay_gain_db,
    };
    return patched;
  });

  return changed ? result : existing;
}

/**
 * Returns true only if pointer is inside the table AND the sort column is BPM or Camelot.
 */
export function shouldFreezeSortPatches(
  isPointerInsideTable: boolean,
  sortBy?: SortBy,
): boolean {
  return Boolean(
    isPointerInsideTable && (sortBy === 'Bpm' || sortBy === 'Camelot'),
  );
}

export function applyPatchesToCaches(patches: ScannerRowPatch[]): void {
  if (!patches || patches.length === 0) return;
  const currentTracks = useLibraryStore.getState().tracks;
  const patchedStore = applyRowPatchesToTracks(currentTracks, patches);
  if (patchedStore !== currentTracks) {
    useLibraryStore.setState({ tracks: patchedStore });
    markTracksRecentlyUpdated(patches.map((p) => p.track_id));
  }
  queryClient.setQueriesData<HydratedFolderCrate | null>(
    { queryKey: ['omkf-folder-crate-detail'] },
    (old) => {
      if (!old) return old;
      const nextTracks = applyRowPatchesToTracks(old.tracks, patches);
      return nextTracks === old.tracks ? old : { ...old, tracks: nextTracks };
    },
  );
}

export function flushFrozenPatches(): void {
  const frozen = useScannerStore.getState().clearFrozenPatches();
  if (frozen.length > 0) {
    applyPatchesToCaches(frozen);
  }
}

export function addTracksToCaches(incoming: Track[]): void {
  if (!incoming || incoming.length === 0) return;
  useLibraryStore.setState((state) => ({
    tracks: mergeIncomingTrackBatch(state.tracks, incoming),
  }));
}

export function removeTracksFromCaches(removedIds: string[]): void {
  if (!removedIds || removedIds.length === 0) return;
  useLibraryStore.setState((state) => ({
    tracks: removeTrackIds(state.tracks, removedIds),
  }));
}

/**
 * Handle Library-related app events, including OMKF capsule updates and scanner telemetry
 */
function LibraryEvents() {
  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    const p1 = listen<ScanProgress>(
      'LibraryScanProgress' satisfies IPCEvent,
      ({ payload }) => {
        LibraryAPI.setRefresh(payload.current, payload.total);
      },
    );

    const p2 = listen<ScannerTelemetry>(
      'hera:scanner-telemetry',
      ({ payload }) => {
        useScannerStore.getState().setTelemetry(payload);
      },
    );

    const p3 = listen<OmkfFolderUpdatedPayload>(
      'hera:omkf-folder-updated',
      ({ payload }) => {
        void queryClient.invalidateQueries({
          queryKey: ['omkf-folder-crates'],
        });
        const crateIds = payload?.crate_ids;
        if (Array.isArray(crateIds) && crateIds.length > 0) {
          for (const cid of crateIds) {
            void queryClient.invalidateQueries({
              queryKey: ['omkf-folder-crate-detail', cid],
            });
          }
        } else {
          void queryClient.invalidateQueries({
            queryKey: ['omkf-folder-crate-detail'],
          });
        }
        if (useLibraryStore.getState().search.trim().length > 0) {
          void queryClient.invalidateQueries({
            queryKey: ['omkf-global-search'],
          });
        }
      },
    );

    const p4 = listen<string[]>(
      'hera:scanner-tracks-removed',
      ({ payload }) => {
        if (Array.isArray(payload) && payload.length > 0) {
          removeTracksFromCaches(payload);
        }
      },
    );

    const p5 = listen<ScannerRowPatch[]>(
      'hera:scanner-rows-updated',
      ({ payload }) => {
        if (!Array.isArray(payload) || payload.length === 0) return;
        if (!player.isPaused()) return;

        const config = queryClient.getQueryData<Config>(configQuery.queryKey);
        const isPointerInside = useScannerStore.getState().isPointerInsideTable;

        if (shouldFreezeSortPatches(isPointerInside, config?.library_sort_by)) {
          useScannerStore.getState().queueFrozenPatches(payload);
        } else {
          applyPatchesToCaches(payload);
        }
      },
    );

    const unsubscribeScanner = useScannerStore.subscribe((state, prevState) => {
      if (prevState.isPointerInsideTable && !state.isPointerInsideTable) {
        flushFrozenPatches();
      }
    });

    const p6 = listen('hera:tray-toggle-mini', () => {
      void useMiniModeStore.getState().toggleMiniMode();
    });

    const p7 = listen('hera:window-restored', () => {
      window.dispatchEvent(new Event('resize'));
    });

    return () => {
      void p1.then((unlisten) => unlisten());
      void p2.then((unlisten) => unlisten());
      void p3.then((unlisten) => unlisten());
      void p4.then((unlisten) => unlisten());
      void p5.then((unlisten) => unlisten());
      void p6.then((unlisten) => unlisten());
      void p7.then((unlisten) => unlisten());
      unsubscribeScanner();
    };
  }, []);

  return null;
}

export default LibraryEvents;

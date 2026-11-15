import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import type { IPCEvent, ScanProgress, Track } from '../generated/typings';
import {
  type ScannerRowPatch,
  type ScannerTelemetry,
  useScannerStore,
} from '../lib/scanner-v3';
import queryClient from '../lib/query-client';
import { allTracksQuery } from '../hooks/useAllTracks';

/**
 * Handle Library-related app events, like refreshing and progress status
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

    const p2 = listen<ScannerTelemetry>('hera:scanner-telemetry', ({ payload }) => {
      useScannerStore.getState().setTelemetry(payload);
    });

    const p3 = listen<ScannerRowPatch[]>('hera:scanner-rows-updated', ({ payload }) => {
      if (!Array.isArray(payload) || payload.length === 0) return;
      const patchMap = new Map<string, ScannerRowPatch>();
      for (const p of payload) {
        patchMap.set(p.track_id, p);
      }
      queryClient.setQueryData<Track[]>(allTracksQuery.queryKey, (old) => {
        if (!old) return old;
        return old.map((track) => {
          const p = patchMap.get(track.id);
          if (!p) return track;
          return {
            ...track,
            bpm: p.bpm ?? track.bpm,
            camelot: p.camelot ?? track.camelot,
            energy: p.energy ?? track.energy,
            scan_level: p.scan_level ?? track.scan_level,
            is_transition_candidate:
              p.is_transition_candidate ?? track.is_transition_candidate,
            is_fake_320kbps: p.is_fake_320kbps ?? track.is_fake_320kbps,
          };
        });
      });
    });

    return () => {
      void p1.then((unlisten) => unlisten());
      void p2.then((unlisten) => unlisten());
      void p3.then((unlisten) => unlisten());
    };
  }, []);

  return null;
}

export default LibraryEvents;

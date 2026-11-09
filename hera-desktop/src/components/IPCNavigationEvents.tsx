import { useNavigate } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useCallback, useEffect } from 'react';
import Keybinding from 'react-keybinding-component';

import { usePlayerState } from '../hooks/usePlayer';
import { goToPlayingTrack } from '../lib/queue-origin';
import { isCtrlKey } from '../lib/utils-events';

/**
 * Event Handlers for Navigation Events, triggered via IPC, typically, from
 * the application menu
 */
export default function IPCNavigationEvents() {
  const navigate = useNavigate();
  const queueOrigin = usePlayerState((state) => state.queueOrigin);

  // Navigation handlers
  const goToLibrary = useCallback(() => {
    void navigate({ to: '/library' });
  }, [navigate]);

  const goToPlaylists = useCallback(() => {
    void navigate({ to: '/playlists' });
  }, [navigate]);

  const goToSettings = useCallback(() => {
    void navigate({ to: '/settings/library' });
  }, [navigate]);

  const goToPlayingTrackOnEvent = useCallback(() => {
    goToPlayingTrack(queueOrigin, navigate);
  }, [queueOrigin, navigate]);

  // Listen to IPC events for navigation
  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    const unlistenerPromises = [
      // Shortcuts from the application menu
      getCurrentWindow().listen('GoToLibrary', () => {
        goToLibrary();
      }),
      getCurrentWindow().listen('GoToPlaylists', goToPlaylists),
      getCurrentWindow().listen('GoToSettings', goToSettings),
      getCurrentWindow().listen('JumpToPlayingTrack', goToPlayingTrackOnEvent),
    ];

    return function cleanup() {
      void Promise.all(unlistenerPromises).then((unlisteners) => {
        unlisteners.forEach((unlisten) => {
          unlisten();
        });
      });
    };
  }, [
    goToLibrary,
    goToPlaylists,
    goToPlayingTrackOnEvent,
    goToSettings,
    navigate,
    queueOrigin,
  ]);

  return (
    <Keybinding
      onKey={(e) => {
        if (isCtrlKey(e)) {
          switch (e.key) {
            case '1':
              goToLibrary();
              break;
            case '2':
              goToPlaylists();
              break;
            case ',':
              goToSettings();
              break;
            default:
              break;
          }
        }
      }}
    />
  );
}

import { useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';
import { createRootRoute, Outlet, useLocation } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { info } from '@tauri-apps/plugin-log';
import { lazy, Suspense, useEffect } from 'react';

import SettingsAPI from '../api/SettingsAPI';
import AppEvents from '../components/AppEvents';
import DropzoneImport from '../components/DropzoneImport';
import { ErrorView, NotFoundView } from '../components/GlobalErrors';
import GlobalKeyBindings from '../components/GlobalKeyBindings';
import Header from '../components/Header';
import IPCNavigationEvents from '../components/IPCNavigationEvents';
import IPCPlayerEvents from '../components/IPCPlayerEvents';
import LibraryEvents from '../components/LibraryEvents';
import Navigation from '../components/Navigation';
import PlayerEvents from '../components/PlayerEvents';
import Toasts from '../components/Toasts';
import useInvalidate from '../hooks/useInvalidate';
import DatabaseBridge from '../lib/bridge-database';
import { useMiniModeStore } from '../lib/mini-mode-store';
import { configQuery } from '../lib/queries';
import queryClient from '../lib/query-client';
import useLibraryStore from '../lib/store';
import { logAndNotifyError } from '../lib/utils';

const LazyMiniOverlay = lazy(() => import('../components/MiniOverlay'));

type Search = {
  jump_to_playing_track?: boolean;
};

export const Route = createRootRoute({
  component: ViewRoot,
  errorComponent: ErrorView,
  notFoundComponent: NotFoundView,
  loader: () => queryClient.ensureQueryData(configQuery),
  validateSearch: (search): Search => {
    const jump_to_playing_track = Boolean(search?.jump_to_playing_track);

    if (jump_to_playing_track === true) {
      return {
        jump_to_playing_track,
      };
    }

    return {};
  },
});

function ViewRoot() {
  const invalidate = useInvalidate();
  const { t } = useLingui();

  useEffect(() => {
    SettingsAPI.init(invalidate).catch(logAndNotifyError);
    void DatabaseBridge.getAllTracks()
      .then((tracks) => {
        useLibraryStore.setState({ tracks });
      })
      .catch(() => {});
  }, [invalidate]);

  const location = useLocation();
  const layoutMode = useMiniModeStore((s) => s.layoutMode);
  const toggleMiniMode = useMiniModeStore((s) => s.toggleMiniMode);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (
        (e.ctrlKey || e.metaKey) &&
        e.shiftKey &&
        e.key.toLowerCase() === 'm'
      ) {
        e.preventDefault();
        void toggleMiniMode();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [toggleMiniMode]);

  useEffect(() => {
    if (isTauri()) {
      info(`Navigated to: ${location.pathname}`).catch(() => {});
    } else {
      console.info(`Navigated to: ${location.pathname}`);
    }
  }, [location]);

  return (
    <div
      {...stylex.props(styles.root)}
      data-museeks-os={
        typeof window !== 'undefined' ? window.__MUSEEKS_PLATFORM : 'windows'
      }
    >
      <IPCNavigationEvents />
      <IPCPlayerEvents />
      <AppEvents />
      <LibraryEvents />
      <PlayerEvents />
      <GlobalKeyBindings />

      {layoutMode !== 'full' ? (
        <Suspense fallback={null}>
          <LazyMiniOverlay />
        </Suspense>
      ) : (
        <>
          <Header />
          <div {...stylex.props(styles.workspaceBody)}>
            <Navigation />
            <main
              aria-label={t`Contenido principal de cabina`}
              {...stylex.props(styles.mainContent)}
            >
              <Outlet />
            </main>
          </div>
          <DropzoneImport />
        </>
      )}

      <Toasts />
    </div>
  );
}

const styles = stylex.create({
  root: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    backgroundColor: 'var(--background)',
    color: 'var(--text)',
  },
  workspaceBody: {
    display: 'flex',
    flexDirection: 'row',
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    minWidth: 0,
    backgroundColor: 'var(--background)',
  },
  mainContent: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    position: 'relative',
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0,
    minWidth: 0,
    backgroundColor: 'var(--background)',
  },
});

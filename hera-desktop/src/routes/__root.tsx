import { useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';
import { useSuspenseQuery } from '@tanstack/react-query';
import {
  createRootRoute,
  Outlet,
  useLocation,
  useNavigate,
} from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { info } from '@tauri-apps/plugin-log';
import { lazy, Suspense, useEffect, useState } from 'react';

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
import OnboardingWizardModal, {
  type ConfigWithBoothFields,
} from '../components/OnboardingWizardModal';
import PlayerEvents from '../components/PlayerEvents';
import Toasts from '../components/Toasts';
import type { Track } from '../generated/typings';
import useInvalidate from '../hooks/useInvalidate';
import { bindAgentGlobalBridge } from '../lib/agent-store';
import { useMiniModeStore } from '../lib/mini-mode-store';
import { registerTrackLoadMetadata } from '../lib/native-audio-transport';
import player from '../lib/player';
import { configQuery } from '../lib/queries';
import queryClient from '../lib/query-client';
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
  const navigate = useNavigate();
  const { t } = useLingui();
  const config = useSuspenseQuery(configQuery).data as ConfigWithBoothFields;
  const shouldAutoPromptOnboarding =
    config.onboarding_completed === false ||
    (config.onboarding_completed === undefined &&
      config.library_folders.length === 0);
  const [onboardingDismissed, setOnboardingDismissed] = useState(false);

  useEffect(() => {
    const onTrackChange = (track: Track | null) => {
      if (track) {
        registerTrackLoadMetadata(
          track.path,
          track.replay_gain_db,
          track.hot_cues?.map((c) => c.seconds),
        );
      }
    };
    player.on('trackChange', onTrackChange);
    return () => {
      player.off('trackChange', onTrackChange);
    };
  }, []);

  useEffect(() => {
    SettingsAPI.init(() => {}).catch(logAndNotifyError);
    bindAgentGlobalBridge({
      invalidate,
      navigate: (to) => {
        void navigate({ to: to as '/library' });
      },
    });
  }, [invalidate, navigate]);

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
        return;
      }
      if (
        (e.ctrlKey || e.metaKey) &&
        !e.shiftKey &&
        !e.altKey &&
        e.key.toLowerCase() === 'j'
      ) {
        e.preventDefault();
        void navigate({ to: '/agent' });
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [navigate, toggleMiniMode]);

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
      <OnboardingWizardModal
        isOpen={shouldAutoPromptOnboarding && !onboardingDismissed}
        onClose={() => setOnboardingDismissed(true)}
        onCompleted={() => setOnboardingDismissed(true)}
      />
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

import { Trans, useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';
import { useQuery } from '@tanstack/react-query';
import {
  createFileRoute,
  Outlet,
  useMatch,
  useNavigate,
} from '@tanstack/react-router';
import type {
  MenuItemOptions,
  PredefinedMenuItemOptions,
} from '@tauri-apps/api/menu';
import { useCallback, useEffect, useMemo } from 'react';

import PlaylistsAPI from '../api/PlaylistsAPI';
import SideNav from '../components/SideNav';
import SideNavLink from '../components/SideNavLink';
import ButtonIcon from '../elements/ButtonIcon';
import Link from '../elements/Link';
import View from '../elements/View';
import * as ViewMessage from '../elements/ViewMessage';
import useInvalidate from '../hooks/useInvalidate';
import DatabaseBridge from '../lib/bridge-database';
import player from '../lib/player';

export const Route = createFileRoute('/playlists')({
  component: ViewPlaylists,
});

function ViewPlaylists() {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const navigate = useNavigate();

  const { data: playlists = [] } = useQuery({
    queryKey: ['playlists'],
    queryFn: () => DatabaseBridge.getAllPlaylists(),
    staleTime: 30_000,
  });

  const childPlaylistMatch = useMatch({
    from: '/playlists/$playlistID',
    shouldThrow: false,
  });

  useEffect(() => {
    if (childPlaylistMatch) return;
    const queueOrigin = player.getQueueOrigin();
    if (queueOrigin?.type === 'playlist') {
      void navigate({
        to: '/playlists/$playlistID',
        params: { playlistID: queueOrigin.playlistID },
        replace: true,
      });
      return;
    }
    if (playlists.length > 0) {
      void navigate({
        to: '/playlists/$playlistID',
        params: { playlistID: playlists[0].id },
        replace: true,
      });
    }
  }, [childPlaylistMatch, navigate, playlists]);

  const createPlaylist = useCallback(async () => {
    const playlist = await PlaylistsAPI.create(t`New playlist`, [], false);

    if (playlist) {
      await invalidate();
      void navigate({
        to: '/playlists/$playlistID',
        params: { playlistID: playlist.id },
      });
    }
  }, [navigate, invalidate, t]);

  const renamePlaylist = useCallback(
    async (playlistID: string, name: string) => {
      await PlaylistsAPI.rename(playlistID, name);
      await invalidate();
    },
    [invalidate],
  );

  const sideNavItems = useMemo(() => {
    return playlists.map((playlist) => {
      const contextMenuItems: Array<
        MenuItemOptions | PredefinedMenuItemOptions
      > = [
        {
          text: t`Delete`,
          action: async () => {
            await PlaylistsAPI.remove(playlist.id);
            await invalidate();
          },
        },
        { item: 'Separator' },
        {
          text: t`Duplicate`,
          action: async () => {
            await PlaylistsAPI.duplicate(playlist.id);
            await invalidate();
          },
        },
        { item: 'Separator' },
        {
          text: t`Export`,
          action: async () => {
            await DatabaseBridge.exportPlaylist(playlist.id);
          },
        },
      ];

      return (
        <SideNavLink
          key={playlist.id}
          label={playlist.name}
          id={playlist.id}
          linkOptions={{
            to: '/playlists/$playlistID',
            params: { playlistID: playlist.id },
          }}
          onRename={renamePlaylist}
          contextMenuItems={contextMenuItems}
        />
      );
    });
  }, [playlists, renamePlaylist, invalidate, t]);

  let playlistContent: React.ReactNode;

  if (childPlaylistMatch) {
    playlistContent = <Outlet />;
  } else if (playlists.length === 0) {
    playlistContent = (
      <ViewMessage.Notice>
        <p>
          <Trans>You haven{"'"}t created any playlist yet</Trans>
        </p>
        <ViewMessage.Sub>
          <Link onClick={createPlaylist}>
            <Trans>create one now</Trans>
          </Link>
        </ViewMessage.Sub>
      </ViewMessage.Notice>
    );
  } else {
    playlistContent = (
      <ViewMessage.Notice>
        <p>
          <Trans>No playlist selected</Trans>
        </p>
      </ViewMessage.Notice>
    );
  }

  const sidebar = (
    <div {...stylex.props(styles.sidebarShell)}>
      <div {...stylex.props(styles.sideNavWrap)}>
        <SideNav
          title={t`Playlists`}
          actions={
            <ButtonIcon
              icon="plus"
              onClick={createPlaylist}
              label={t`New Playlist`}
            />
          }
        >
          {sideNavItems}
        </SideNav>
      </div>
    </div>
  );

  return <View sideNav={sidebar}>{playlistContent}</View>;
}

const styles = stylex.create({
  sidebarShell: {
    display: 'flex',
    flexDirection: 'column',
    width: '220px',
    flexShrink: 0,
    backgroundColor: 'var(--sidebar-bg)',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: 'var(--border-color)',
    height: '100%',
    minHeight: 0,
  },
  sideNavWrap: {
    flexGrow: 1,
    minHeight: 0,
    display: 'flex',
  },
});

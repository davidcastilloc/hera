import { Trans, useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';
import {
  createFileRoute,
  Link as RouterLink,
  Outlet,
  redirect,
  useMatch,
  useNavigate,
} from '@tanstack/react-router';
import type {
  MenuItemOptions,
  PredefinedMenuItemOptions,
} from '@tauri-apps/api/menu';
import { useCallback, useEffect, useMemo, useState } from 'react';

import PlaylistsAPI from '../api/PlaylistsAPI';
import SideNav from '../components/SideNav';
import SideNavLink from '../components/SideNavLink';
import ButtonIcon from '../elements/ButtonIcon';
import Link from '../elements/Link';
import View from '../elements/View';
import * as ViewMessage from '../elements/ViewMessage';
import useInvalidate from '../hooks/useInvalidate';
import DatabaseBridge from '../lib/bridge-database';
import {
  formatFolderCrateSubtitle,
  isFolderPlaylistId,
  listFolderCrates,
  toFolderPlaylistId,
} from '../lib/omkf-bridge';
import player from '../lib/player';

export const Route = createFileRoute('/playlists')({
  component: ViewPlaylists,
  beforeLoad: async ({ params }) => {
    const [playlists, folderCrates] = await Promise.all([
      DatabaseBridge.getAllPlaylists(),
      listFolderCrates(),
    ]);
    const queueOrigin = player.getQueueOrigin();

    if (!('playlistID' in params)) {
      if (queueOrigin?.type === 'playlist') {
        throw redirect({
          to: '/playlists/$playlistID',
          params: { playlistID: queueOrigin.playlistID },
        });
      }
      if (folderCrates.length > 0) {
        throw redirect({
          to: '/playlists/$playlistID',
          params: { playlistID: toFolderPlaylistId(folderCrates[0].crate_id) },
        });
      }
      if (playlists.length > 0) {
        throw redirect({
          to: '/playlists/$playlistID',
          params: { playlistID: playlists[0].id },
        });
      }
    }

    return { playlists, folderCrates };
  },
  loader: async ({ context }) => {
    return context;
  },
});

function ViewPlaylists() {
  const { playlists, folderCrates } = Route.useLoaderData();
  const { t } = useLingui();

  const invalidate = useInvalidate();
  const navigate = useNavigate();

  const childPlaylistMatch = useMatch({
    from: '/playlists/$playlistID',
    shouldThrow: false,
  });
  const activePlaylistId = childPlaylistMatch?.params.playlistID ?? null;
  const [activeTab, setActiveTab] = useState<'folders' | 'playlists'>(() =>
    activePlaylistId && !isFolderPlaylistId(activePlaylistId)
      ? 'playlists'
      : 'folders',
  );

  useEffect(() => {
    if (activePlaylistId) {
      setActiveTab(
        isFolderPlaylistId(activePlaylistId) ? 'folders' : 'playlists',
      );
    }
  }, [activePlaylistId]);

  const createPlaylist = useCallback(async () => {
    const playlist = await PlaylistsAPI.create(t`New playlist`, [], false);

    if (playlist) {
      setActiveTab('playlists');
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
  } else if (activeTab === 'folders') {
    playlistContent =
      folderCrates.length === 0 ? (
        <ViewMessage.Notice>
          <p>No hay carpetas (.hera) indexadas todavía</p>
          <ViewMessage.Sub>
            Escanea una carpeta en tu{' '}
            <Link linkOptions={{ to: '/library' }}>Biblioteca de Cabina</Link> o
            crea una <Link onClick={createPlaylist}>playlist manual</Link>.
          </ViewMessage.Sub>
        </ViewMessage.Notice>
      ) : (
        <ViewMessage.Notice>
          <p>Selecciona una carpeta (.hera) para abrir sus reglas y pistas</p>
        </ViewMessage.Notice>
      );
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
      <div
        role="tablist"
        aria-label="Origen de crates"
        {...stylex.props(styles.tabBar)}
      >
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'folders'}
          onClick={() => setActiveTab('folders')}
          {...stylex.props(
            styles.tabBtn,
            activeTab === 'folders' && styles.tabBtnActive,
          )}
        >
          Carpetas (.hera)
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'playlists'}
          onClick={() => setActiveTab('playlists')}
          {...stylex.props(
            styles.tabBtn,
            activeTab === 'playlists' && styles.tabBtnActive,
          )}
        >
          Playlists
        </button>
      </div>

      {activeTab === 'folders' ? (
        <div {...stylex.props(styles.foldersPanel)} data-museeks-list>
          <div {...stylex.props(styles.foldersHeader)}>
            <span {...stylex.props(styles.foldersTitle)}>Carpetas (.hera)</span>
            <span {...stylex.props(styles.foldersCount)}>
              {folderCrates.length}
            </span>
          </div>
          {folderCrates.length === 0 ? (
            <p {...stylex.props(styles.foldersEmpty)}>
              Sin cápsulas .hera detectadas aún.
            </p>
          ) : (
            <div {...stylex.props(styles.foldersList)}>
              {folderCrates.map((crate) => {
                const folderId = toFolderPlaylistId(crate.crate_id);
                const isActive = activePlaylistId === folderId;
                return (
                  <RouterLink
                    key={crate.crate_id}
                    to="/playlists/$playlistID"
                    params={{ playlistID: folderId }}
                    draggable={false}
                    {...stylex.props(
                      styles.folderLink,
                      isActive && styles.folderLinkActive,
                    )}
                  >
                    <div {...stylex.props(styles.folderTopRow)}>
                      <span {...stylex.props(styles.folderName)}>
                        {crate.display_name}
                      </span>
                      <span {...stylex.props(styles.folderTracks)}>
                        {crate.track_count}
                      </span>
                    </div>
                    <span {...stylex.props(styles.folderSub)}>
                      {formatFolderCrateSubtitle(crate)}
                    </span>
                  </RouterLink>
                );
              })}
            </div>
          )}
        </div>
      ) : (
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
      )}
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
  tabBar: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    columnGap: '4px',
    paddingBlock: '8px',
    paddingInline: '8px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    backgroundColor: '#10141b',
    flexShrink: 0,
  },
  tabBtn: {
    paddingBlock: '5px',
    paddingInline: '6px',
    fontSize: '11px',
    fontWeight: 500,
    color: '#8e98a8',
    backgroundColor: 'transparent',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'transparent',
    borderRadius: '6px',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  tabBtnActive: {
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderColor: 'rgba(20, 184, 166, 0.35)',
    fontWeight: 600,
  },
  foldersPanel: {
    flexGrow: 1,
    minHeight: 0,
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
  },
  foldersHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '10px',
    paddingInline: '12px',
  },
  foldersTitle: {
    fontSize: '12px',
    fontWeight: 700,
    color: '#8e98a8',
  },
  foldersCount: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  foldersEmpty: {
    fontSize: '12px',
    color: '#64748b',
    paddingInline: '12px',
    margin: 0,
  },
  foldersList: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
    paddingInline: '8px',
    paddingBottom: '12px',
  },
  folderLink: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    paddingBlock: '7px',
    paddingInline: '9px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'transparent',
    textDecoration: 'none',
    color: '#edeff3',
  },
  folderLinkActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderColor: 'rgba(20, 184, 166, 0.32)',
  },
  folderTopRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '6px',
  },
  folderName: {
    fontSize: '12.5px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  folderTracks: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  folderSub: {
    fontSize: '10.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  sideNavWrap: {
    flexGrow: 1,
    minHeight: 0,
    display: 'flex',
  },
});

import { useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';
import { useSuspenseQuery } from '@tanstack/react-query';
import {
  createFileRoute,
  redirect,
  useLoaderData,
} from '@tanstack/react-router';
import { useCallback, useMemo } from 'react';

import PlaylistsAPI from '../api/PlaylistsAPI';
import CrateRulesBanner from '../components/CrateRulesBanner';
import HeraDjToolbar from '../components/HeraDjToolbar';
import TrackList from '../components/TrackList';
import Link from '../elements/Link';
import * as ViewMessage from '../elements/ViewMessage';
import type { Playlist, Track } from '../generated/typings';
import useFilteredTracks from '../hooks/useFilteredTracks';
import useGlobalTrackListStatus from '../hooks/useGlobalTrackListStatus';
import useInvalidate from '../hooks/useInvalidate';
import DatabaseBridge from '../lib/bridge-database';
import {
  type FolderCrateDetail,
  fromFolderPlaylistId,
  isFolderPlaylistId,
  readFolderCrate,
} from '../lib/omkf-bridge';
import { configQuery } from '../lib/queries';
import { promoteCrateFull } from '../lib/scanner-v3';
import useLibraryStore from '../lib/store';
import type { QueueOrigin } from '../types/museeks';

type LoaderPayload =
  | {
      kind: 'folder';
      crateDetail: FolderCrateDetail;
      playlist: null;
      playlistTracks: Track[];
    }
  | {
      kind: 'playlist';
      crateDetail: null;
      playlist: Playlist;
      playlistTracks: Track[];
    };

export const Route = createFileRoute('/playlists/$playlistID')({
  component: ViewPlaylistDetails,
  loader: async ({ params }): Promise<LoaderPayload> => {
    if (isFolderPlaylistId(params.playlistID)) {
      const crateId = fromFolderPlaylistId(params.playlistID);
      const crateDetail = await readFolderCrate(crateId);
      if (!crateDetail) {
        throw redirect({ to: '/playlists' });
      }
      void promoteCrateFull(
        crateId,
        crateDetail.tracks.map((t) => t.id),
      );
      return {
        kind: 'folder',
        crateDetail,
        playlist: null,
        playlistTracks: crateDetail.tracks,
      };
    }

    try {
      const playlist = await DatabaseBridge.getPlaylist(params.playlistID);
      const playlistTracks = await DatabaseBridge.getTracks(playlist.tracks);

      return {
        kind: 'playlist',
        crateDetail: null,
        playlist,
        playlistTracks,
      };
    } catch (err) {
      if (err === 'Playlist not found') {
        throw redirect({ to: '/playlists' });
      }

      throw err;
    }
  },
});

function ViewPlaylistDetails() {
  const loaderData = Route.useLoaderData();
  const { playlists } = useLoaderData({ from: '/playlists' });
  const { playlistID } = Route.useParams();
  const config = useSuspenseQuery(configQuery).data;

  const { t } = useLingui();
  const invalidate = useInvalidate();

  const search = useLibraryStore((state) => state.search);
  const filteredTracks = useFilteredTracks(loaderData.playlistTracks);
  useGlobalTrackListStatus(filteredTracks);

  const queueOrigin = useMemo(() => {
    return { type: 'playlist', playlistID } satisfies QueueOrigin;
  }, [playlistID]);

  const onReorder = useCallback(
    async (tracks: Track[]) => {
      if (playlistID != null && loaderData.kind === 'playlist') {
        await PlaylistsAPI.reorderTracks(playlistID, tracks);
        await invalidate();
      }
    },
    [invalidate, playlistID, loaderData.kind],
  );

  const extraContextMenu = useMemo(() => {
    if (loaderData.kind === 'folder') return undefined;
    return [
      {
        label: t`Remove from playlist`,
        action: async (selectedTracks: Set<string>) => {
          await PlaylistsAPI.removeTracks(
            playlistID,
            Array.from(selectedTracks),
          );
          await invalidate();
        },
      },
    ];
  }, [playlistID, invalidate, t, loaderData.kind]);

  const displayName =
    loaderData.kind === 'folder'
      ? loaderData.crateDetail.summary.display_name
      : loaderData.playlist.name;

  if (
    loaderData.playlistTracks.length === 0 &&
    loaderData.kind === 'playlist'
  ) {
    return (
      <ViewMessage.Notice>
        <p>Esta playlist aún no tiene pistas</p>
        <ViewMessage.Sub>
          Añade pistas desde la{' '}
          <Link linkOptions={{ to: '/library' }}>Biblioteca de Cabina</Link>
        </ViewMessage.Sub>
      </ViewMessage.Notice>
    );
  }

  return (
    <div {...stylex.props(styles.playlistContainer)}>
      <HeraDjToolbar
        title={displayName}
        tracksOverride={filteredTracks}
        crateName={displayName}
      />
      {loaderData.kind === 'folder' && (
        <CrateRulesBanner
          crate={loaderData.crateDetail.summary}
          rulesMarkdown={loaderData.crateDetail.rules_markdown}
          ledgerMarkdown={loaderData.crateDetail.ledger_markdown}
          onOrganized={invalidate}
        />
      )}
      {filteredTracks.length === 0 && search.length > 0 ? (
        <ViewMessage.Notice>
          <p>No hay resultados para tu búsqueda en este crate</p>
        </ViewMessage.Notice>
      ) : loaderData.playlistTracks.length === 0 ? (
        <ViewMessage.Notice>
          <p>Esta carpeta aún no tiene pistas indexadas</p>
        </ViewMessage.Notice>
      ) : (
        <div {...stylex.props(styles.listViewport)}>
          <TrackList
            layout="default"
            data={filteredTracks}
            tracksDensity={config.track_view_density}
            playlists={playlists}
            queueOrigin={queueOrigin}
            onReorder={loaderData.kind === 'playlist' ? onReorder : undefined}
            reorderable={loaderData.kind === 'playlist'}
            extraContextMenu={extraContextMenu}
          />
        </div>
      )}
    </div>
  );
}

const styles = stylex.create({
  playlistContainer: {
    height: '100%',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  listViewport: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    display: 'flex',
    flexDirection: 'column',
  },
});

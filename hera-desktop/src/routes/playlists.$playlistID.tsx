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
import HeraDjToolbar from '../components/HeraDjToolbar';
import TrackList from '../components/TrackList';
import Link from '../elements/Link';
import * as ViewMessage from '../elements/ViewMessage';
import type { Track } from '../generated/typings';
import useFilteredTracks from '../hooks/useFilteredTracks';
import useGlobalTrackListStatus from '../hooks/useGlobalTrackListStatus';
import useInvalidate from '../hooks/useInvalidate';
import DatabaseBridge from '../lib/bridge-database';
import { configQuery } from '../lib/queries';
import useLibraryStore from '../lib/store';
import type { QueueOrigin } from '../types/museeks';

export const Route = createFileRoute('/playlists/$playlistID')({
  component: ViewPlaylistDetails,
  loader: async ({ params }) => {
    try {
      const playlist = await DatabaseBridge.getPlaylist(params.playlistID);
      const playlistTracks = await DatabaseBridge.getTracks(playlist.tracks);

      return { playlist, playlistTracks };
    } catch (err) {
      if (err === 'Playlist not found') {
        throw redirect({ to: '/playlists' });
      }

      throw err;
    }
  },
});

function ViewPlaylistDetails() {
  const { playlist, playlistTracks } = Route.useLoaderData();
  const { playlists } = useLoaderData({ from: '/playlists' });
  const { playlistID } = Route.useParams();
  const config = useSuspenseQuery(configQuery).data;

  const { t } = useLingui();

  const invalidate = useInvalidate();

  const search = useLibraryStore((state) => state.search);
  const filteredTracks = useFilteredTracks(playlistTracks);
  useGlobalTrackListStatus(filteredTracks);

  const queueOrigin = useMemo(() => {
    return { type: 'playlist', playlistID } satisfies QueueOrigin;
  }, [playlistID]);

  const onReorder = useCallback(
    async (tracks: Track[]) => {
      if (playlistID != null) {
        await PlaylistsAPI.reorderTracks(playlistID, tracks);
        await invalidate();
      }
    },
    [invalidate, playlistID],
  );

  const extraContextMenu = useMemo(() => {
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
  }, [playlistID, invalidate, t]);

  if (playlistTracks.length === 0) {
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

  if (filteredTracks.length === 0 && search.length > 0) {
    return (
      <ViewMessage.Notice>
        <p>No hay resultados para tu búsqueda en esta playlist</p>
      </ViewMessage.Notice>
    );
  }

  return (
    <div {...stylex.props(styles.playlistContainer)}>
      <HeraDjToolbar
        title={playlist.name}
        tracksOverride={filteredTracks}
        crateName={playlist.name}
      />
      <div {...stylex.props(styles.listViewport)}>
        <TrackList
          layout="default"
          data={filteredTracks}
          tracksDensity={config.track_view_density}
          playlists={playlists}
          queueOrigin={queueOrigin}
          onReorder={onReorder}
          reorderable={true}
          extraContextMenu={extraContextMenu}
        />
      </div>
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

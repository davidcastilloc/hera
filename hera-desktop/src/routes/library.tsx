import * as stylex from '@stylexjs/stylex';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect } from 'react';

import HeraDjToolbar from '../components/HeraDjToolbar';
import TrackList from '../components/TrackList';
import TrackListStates from '../components/TrackListStates';
import View from '../elements/View';
import useFilteredTracks from '../hooks/useFilteredTracks';
import useGlobalTrackListStatus from '../hooks/useGlobalTrackListStatus';
import DatabaseBridge from '../lib/bridge-database';
import { allTracksQuery, configQuery } from '../lib/queries';
import queryClient from '../lib/query-client';
import useLibraryStore from '../lib/store';
import type { QueueOrigin } from '../types/museeks';

const QUEUE_ORIGIN: QueueOrigin = { type: 'library' };

export const Route = createFileRoute('/library')({
  component: ViewLibrary,
  loader: async () => {
    const [_, playlists] = await Promise.all([
      queryClient.prefetchQuery(allTracksQuery),
      DatabaseBridge.getAllPlaylists(),
    ]);

    return { playlists };
  },
});

function ViewLibrary() {
  const { playlists } = Route.useLoaderData();

  const config = useSuspenseQuery(configQuery).data;
  const { data: tracks, isLoading } = useQuery(allTracksQuery);

  useEffect(() => {
    if (tracks) {
      useLibraryStore.setState({ tracks });
    }
  }, [tracks]);

  const filteredTracks = useFilteredTracks(
    tracks ?? [],
    config.library_sort_by,
    config.library_sort_order,
  );
  useGlobalTrackListStatus(filteredTracks);

  return (
    <View xstyle={styles.libraryView}>
      <HeraDjToolbar
        title="Biblioteca"
        tracksOverride={filteredTracks}
        crateName="Hera_Active_Set"
      />
      <div {...stylex.props(styles.listViewport)}>
        <TrackListStates isLoading={isLoading} tracks={filteredTracks}>
          <TrackList
            layout="default"
            data={filteredTracks}
            queueOrigin={QUEUE_ORIGIN}
            tracksDensity={config.track_view_density}
            playlists={playlists}
          />
        </TrackListStates>
      </div>
    </View>
  );
}

const styles = stylex.create({
  libraryView: {
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  listViewport: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    position: 'relative',
    display: 'flex',
    flexDirection: 'column',
  },
});

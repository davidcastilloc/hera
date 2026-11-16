import * as stylex from '@stylexjs/stylex';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useMemo, useState } from 'react';

import FolderCrateTree from '../components/FolderCrateTree';
import HeraDjToolbar from '../components/HeraDjToolbar';
import TrackList from '../components/TrackList';
import TrackListStates from '../components/TrackListStates';
import View from '../elements/View';
import useFilteredTracks from '../hooks/useFilteredTracks';
import useGlobalTrackListStatus from '../hooks/useGlobalTrackListStatus';
import DatabaseBridge from '../lib/bridge-database';
import { listFolderCrates } from '../lib/omkf-bridge';
import { allTracksQuery, configQuery } from '../lib/queries';
import queryClient from '../lib/query-client';
import { type TrackWithScanMeta, useScannerStore } from '../lib/scanner-v3';
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
  const [mixesFilter, setMixesFilter] = useState<
    'all' | 'under10m' | 'over10m'
  >('under10m');
  const [selectedCrateId, setSelectedCrateId] = useState<string | null>(null);
  const [crateTreeCollapsed, setCrateTreeCollapsed] = useState(true);
  const sonicTwins = useScannerStore((s) => s.sonicTwins);

  const config = useSuspenseQuery(configQuery).data;
  const { data: tracks, isLoading } = useQuery(allTracksQuery);
  const { data: folderCrates = [] } = useQuery({
    queryKey: ['omkf-folder-crates'],
    queryFn: listFolderCrates,
  });

  useEffect(() => {
    if (tracks) {
      useLibraryStore.setState({ tracks });
    }
  }, [tracks]);

  const sortedTracks = useFilteredTracks(
    tracks ?? [],
    config.library_sort_by,
    config.library_sort_order,
  );

  const selectedCrate = useMemo(
    () => folderCrates.find((c) => c.crate_id === selectedCrateId) ?? null,
    [folderCrates, selectedCrateId],
  );

  const filteredTracks = useMemo(() => {
    let base = sortedTracks;

    if (selectedCrateId) {
      const cleanFolder = selectedCrate?.folder_path
        ? selectedCrate.folder_path
            .replace(/\\/g, '/')
            .replace(/\/+$/, '')
            .toLowerCase()
        : null;
      base = base.filter((t) => {
        const meta = t as TrackWithScanMeta;
        if (meta.crate_id === selectedCrateId) return true;
        if (cleanFolder) {
          const p = t.path.replace(/\\/g, '/').toLowerCase();
          const lastSlash = p.lastIndexOf('/');
          const parentDir = lastSlash >= 0 ? p.slice(0, lastSlash) : '';
          return parentDir === cleanFolder;
        }
        return false;
      });
    }

    if (sonicTwins) {
      const allowedIds = new Set<string>([
        sonicTwins.sourceTrackId,
        ...sonicTwins.matches.map((m) => m.track_id),
      ]);
      base = base.filter((t) => allowedIds.has(t.id));
    }

    if (mixesFilter === 'all') return base;
    return base.filter((t) => {
      const isMix =
        t.is_mix_over_10m === true || (t.duration != null && t.duration > 600);
      return mixesFilter === 'over10m' ? isMix : !isMix;
    });
  }, [sortedTracks, mixesFilter, selectedCrateId, selectedCrate, sonicTwins]);

  useGlobalTrackListStatus(filteredTracks);

  return (
    <View xstyle={styles.libraryView}>
      <HeraDjToolbar
        title={selectedCrate ? selectedCrate.display_name : 'Biblioteca'}
        tracksOverride={filteredTracks}
        crateName={
          selectedCrate ? selectedCrate.display_name : 'Hera_Active_Set'
        }
        mixesFilter={mixesFilter}
        onMixesFilterChange={setMixesFilter}
      />
      <div {...stylex.props(styles.bodyRow)}>
        {folderCrates.length > 0 && (
          <FolderCrateTree
            crates={folderCrates}
            selectedCrateId={selectedCrateId}
            onSelectCrate={setSelectedCrateId}
            collapsed={crateTreeCollapsed}
            onToggleCollapsed={() => setCrateTreeCollapsed((c) => !c)}
            totalTrackCount={tracks?.length ?? 0}
          />
        )}
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
  bodyRow: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    minWidth: 0,
    display: 'flex',
    flexDirection: 'row',
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

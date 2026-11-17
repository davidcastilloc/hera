import * as stylex from '@stylexjs/stylex';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useState } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import CrateRulesBanner from '../components/CrateRulesBanner';
import FolderCrateTree from '../components/FolderCrateTree';
import FolderExplorerHeader from '../components/FolderExplorerHeader';
import HeraDjToolbar from '../components/HeraDjToolbar';
import TrackList from '../components/TrackList';
import TrackListStates from '../components/TrackListStates';
import View from '../elements/View';
import useFilteredTracks from '../hooks/useFilteredTracks';
import useGlobalTrackListStatus from '../hooks/useGlobalTrackListStatus';
import DatabaseBridge from '../lib/bridge-database';
import {
  formatFolderCrateSubtitle,
  listFolderCrates,
  normalizeFolderHierarchy,
  pickInitialFolderCrateId,
  readFolderCrate,
  searchAllFoldersDeduplicated,
} from '../lib/omkf-bridge';
import { configQuery } from '../lib/queries';
import { useScannerStore } from '../lib/scanner-v3';
import useLibraryStore from '../lib/store';
import type { QueueOrigin } from '../types/museeks';

const QUEUE_ORIGIN: QueueOrigin = { type: 'library' };

export const Route = createFileRoute('/library')({
  component: ViewLibrary,
});

function ViewLibrary() {
  const [mixesFilter, setMixesFilter] = useState<
    'all' | 'under10m' | 'over10m'
  >('under10m');
  const [selectedCrateId, setSelectedCrateId] = useState<string | null>(null);
  const [crateTreeCollapsed, setCrateTreeCollapsed] = useState(false);

  const sonicTwins = useScannerStore((s) => s.sonicTwins);
  const rawSearch = useLibraryStore((s) => s.search);
  const trimmedSearch = rawSearch.trim();
  const isGlobalSearch = trimmedSearch.length > 0;

  const config = useSuspenseQuery(configQuery).data;

  const { data: playlists = [] } = useQuery({
    queryKey: ['playlists'],
    queryFn: () => DatabaseBridge.getAllPlaylists(),
    staleTime: 30_000,
  });

  const {
    data: folderCrates = [],
    isLoading: isCratesLoading,
    refetch: refetchFolderCrates,
  } = useQuery({
    queryKey: ['omkf-folder-crates'],
    queryFn: listFolderCrates,
    staleTime: 30_000,
  });

  const handleSelectCrate = useCallback((crateId: string) => {
    setSelectedCrateId(crateId);
    useLibraryStore.setState({ setSpineOrderedIds: null });
    if (useLibraryStore.getState().search.trim().length > 0) {
      LibraryAPI.search('');
    }
  }, []);

  useEffect(() => {
    if (folderCrates.length === 0) {
      if (selectedCrateId !== null) {
        setSelectedCrateId(null);
      }
      return;
    }
    const stillValid = selectedCrateId
      ? folderCrates.some((c) => c.crate_id === selectedCrateId)
      : false;
    if (!stillValid) {
      setSelectedCrateId(pickInitialFolderCrateId(folderCrates));
    }
  }, [folderCrates, selectedCrateId]);

  const {
    data: selectedCrateDetail,
    isLoading: isDetailLoading,
    refetch: refetchSelectedCrate,
  } = useQuery({
    queryKey: ['omkf-folder-crate-detail', selectedCrateId],
    queryFn: () =>
      selectedCrateId
        ? readFolderCrate(selectedCrateId)
        : Promise.resolve(null),
    enabled: Boolean(selectedCrateId) && !isGlobalSearch,
    staleTime: 15_000,
  });

  const { data: globalSearchTracks = [], isLoading: isSearchLoading } =
    useQuery({
      queryKey: ['omkf-global-search', trimmedSearch],
      queryFn: () => searchAllFoldersDeduplicated(trimmedSearch),
      enabled: isGlobalSearch,
      staleTime: 10_000,
    });

  const activeFolderTracks = useMemo(
    () =>
      isGlobalSearch ? globalSearchTracks : (selectedCrateDetail?.tracks ?? []),
    [globalSearchTracks, isGlobalSearch, selectedCrateDetail],
  );

  useEffect(() => {
    useLibraryStore.setState({ tracks: activeFolderTracks });
  }, [activeFolderTracks]);

  const sortedTracks = useFilteredTracks(
    activeFolderTracks,
    config.library_sort_by,
    config.library_sort_order,
    isGlobalSearch,
  );

  const selectedCrate = useMemo(
    () => folderCrates.find((c) => c.crate_id === selectedCrateId) ?? null,
    [folderCrates, selectedCrateId],
  );

  const activeSubfolders = useMemo(() => {
    const detailSubfolders = selectedCrateDetail?.detail.subfolders;
    if (detailSubfolders && detailSubfolders.length > 0) {
      return detailSubfolders;
    }
    if (!selectedCrateId || folderCrates.length === 0) return [];
    const { childrenByParent } = normalizeFolderHierarchy(folderCrates);
    return childrenByParent.get(selectedCrateId) ?? [];
  }, [folderCrates, selectedCrateDetail, selectedCrateId]);

  const filteredTracks = useMemo(() => {
    let base = sortedTracks;

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
  }, [sortedTracks, mixesFilter, sonicTwins]);

  useGlobalTrackListStatus(filteredTracks);

  const isLoading = isGlobalSearch
    ? isSearchLoading
    : isCratesLoading || (Boolean(selectedCrateId) && isDetailLoading);

  const showContainerSubfolderBoard =
    !isGlobalSearch &&
    !isLoading &&
    filteredTracks.length === 0 &&
    activeSubfolders.length > 0;

  const toolbarTitle = isGlobalSearch
    ? 'Búsqueda en Carpetas'
    : selectedCrate
      ? selectedCrate.display_name
      : 'Carpetas';

  return (
    <View xstyle={styles.libraryView}>
      <HeraDjToolbar
        title={toolbarTitle}
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
            onSelectCrate={handleSelectCrate}
            collapsed={crateTreeCollapsed}
            onToggleCollapsed={() => setCrateTreeCollapsed((c) => !c)}
          />
        )}
        <div {...stylex.props(styles.listViewport)}>
          <FolderExplorerHeader
            crates={folderCrates}
            selectedCrateId={selectedCrateId}
            subfolders={activeSubfolders}
            trackCount={filteredTracks.length}
            searchQuery={rawSearch}
            onSelectCrate={handleSelectCrate}
            onClearSearch={() => LibraryAPI.search('')}
          >
            {selectedCrate && (
              <CrateRulesBanner
                crate={selectedCrateDetail?.detail.summary ?? selectedCrate}
                rulesMarkdown={selectedCrateDetail?.detail.rules_markdown ?? ''}
                ledgerEntries={selectedCrateDetail?.detail.ledger_entries ?? []}
                onOrganized={async () => {
                  await Promise.all([
                    refetchFolderCrates(),
                    refetchSelectedCrate(),
                  ]);
                }}
              />
            )}
          </FolderExplorerHeader>

          {showContainerSubfolderBoard ? (
            <div {...stylex.props(styles.containerFolderBoard)}>
              <div {...stylex.props(styles.containerFolderCard)}>
                <h2 {...stylex.props(styles.containerTitle)}>
                  {selectedCrate?.display_name ?? 'Carpeta contenedora'}
                </h2>
                <p {...stylex.props(styles.containerSubtitle)}>
                  Esta raíz agrupa {activeSubfolders.length} subcarpeta(s).
                  Elige una subcarpeta para abrir sus pistas y cápsula{' '}
                  <code>.hera</code>.
                </p>
                <div {...stylex.props(styles.containerSubfolderList)}>
                  {activeSubfolders.map((sub) => (
                    <button
                      key={sub.crate_id}
                      type="button"
                      onClick={() => handleSelectCrate(sub.crate_id)}
                      {...stylex.props(styles.containerSubfolderBtn)}
                    >
                      <span {...stylex.props(styles.containerSubName)}>
                        {sub.display_name}
                      </span>
                      <span {...stylex.props(styles.containerSubMeta)}>
                        {sub.track_count > 0
                          ? `${sub.track_count} pistas · ${formatFolderCrateSubtitle(sub)}`
                          : `${sub.subfolder_count ?? 0} subcarpetas`}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          ) : (
            <TrackListStates isLoading={isLoading} tracks={filteredTracks}>
              <TrackList
                layout="default"
                data={filteredTracks}
                queueOrigin={QUEUE_ORIGIN}
                tracksDensity={config.track_view_density}
                playlists={playlists}
              />
            </TrackListStates>
          )}
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
    minWidth: 0,
    position: 'relative',
    display: 'flex',
    flexDirection: 'column',
  },
  containerFolderBoard: {
    flexGrow: 1,
    minHeight: 0,
    overflowY: 'auto',
    padding: '24px',
    backgroundColor: '#0c0e12',
    display: 'flex',
    alignItems: 'flex-start',
    justifyContent: 'center',
  },
  containerFolderCard: {
    width: '100%',
    maxWidth: '640px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    paddingBlock: '20px',
    paddingInline: '22px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  containerTitle: {
    margin: 0,
    fontSize: '16px',
    fontWeight: 600,
    color: '#edeff3',
  },
  containerSubtitle: {
    margin: 0,
    fontSize: '12.5px',
    color: '#8e98a8',
    lineHeight: 1.5,
  },
  containerSubfolderList: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    marginTop: '4px',
  },
  containerSubfolderBtn: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
    paddingBlock: '9px',
    paddingInline: '12px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#232a36',
      ':hover': 'rgba(20, 184, 166, 0.45)',
    },
    backgroundColor: {
      default: '#171c24',
      ':hover': '#1b222d',
    },
    color: '#edeff3',
    cursor: 'pointer',
    textAlign: 'left',
  },
  containerSubName: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#edeff3',
  },
  containerSubMeta: {
    fontSize: '11.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
});

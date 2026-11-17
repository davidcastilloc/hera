import { isTauri } from '@tauri-apps/api/core';

import ConfigBridge from '../lib/bridge-config';
import DatabaseBridge from '../lib/bridge-database';
import { syncMockCatalogTracks } from '../lib/omkf-bridge';
import player from '../lib/player';
import queryClient from '../lib/query-client';
import useLibraryStore from '../lib/store';
import toastManager from '../lib/toast-manager';
import { logAndNotifyError } from '../lib/utils';
import { removeRedundantFolders } from '../lib/utils-library';
import type { TrackListStatusInfo, TrackMutation } from '../types/museeks';

async function invalidateOmkfCatalogQueries(): Promise<void> {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: ['omkf-folder-crates'] }),
    queryClient.invalidateQueries({ queryKey: ['omkf-folder-crate-detail'] }),
    queryClient.invalidateQueries({ queryKey: ['omkf-global-search'] }),
    queryClient.invalidateQueries({ queryKey: ['playlists'] }),
    queryClient.invalidateQueries({ queryKey: ['config'] }),
  ]);
}

/**
 * Filter tracks by search
 */
function search(value: string): void {
  useLibraryStore.setState({ search: value });
}

function extractParentDirectory(filePath: string): string | null {
  const normalized = filePath.trim();
  const lastBackslash = normalized.lastIndexOf('\\');
  const lastSlash = normalized.lastIndexOf('/');
  const splitIdx = Math.max(lastBackslash, lastSlash);
  if (splitIdx <= 0) {
    return null;
  }
  return normalized.slice(0, splitIdx);
}

/**
 * Scan the library for new tracks
 */
async function scan(refresh = false): Promise<void> {
  try {
    useLibraryStore.setState({ refreshing: true });

    const libraryFolders = await ConfigBridge.get('library_folders');
    if (libraryFolders.length === 0) {
      toastManager.add({
        title:
          'Añade primero una carpeta o archivos de música para analizar tu biblioteca.',
        type: 'warning',
        timeout: 4500,
      });
      return;
    }

    const scanResult = await DatabaseBridge.importTracks(
      libraryFolders,
      refresh,
    );

    let mockCount = 0;
    if (!isTauri()) {
      const mockTracks = await DatabaseBridge.getAllTracks();
      mockCount = mockTracks.length;
      syncMockCatalogTracks(mockTracks);
      if (
        mockTracks.length > 0 ||
        useLibraryStore.getState().tracks.length === 0
      ) {
        useLibraryStore.setState({ tracks: mockTracks });
      }
    }

    await invalidateOmkfCatalogQueries();

    if (scanResult.track_count > 0) {
      const message = refresh
        ? `Se actualizaron ${scanResult.track_count} pista(s) con análisis DSP y 4 Hot Cues.`
        : `Se importaron ${scanResult.track_count} pista(s) a tus carpetas de cabina.`;

      toastManager.add({ title: message, type: 'success', timeout: 5000 });
    } else if (scanResult.playlist_count === 0) {
      const currentTrackCount = Math.max(
        mockCount,
        useLibraryStore.getState().tracks.length,
      );
      toastManager.add({
        title:
          currentTrackCount === 0 && !isTauri()
            ? 'No se encontraron archivos compatibles (MP3, WAV, FLAC, AIFF, M4A, MP4, MOV, WEBM, MKV) en la carpeta seleccionada.'
            : 'Carpetas sincronizadas en cabina.',
        type: currentTrackCount === 0 && !isTauri() ? 'warning' : 'info',
        timeout: 5000,
      });
    }

    if (scanResult.playlist_count > 0) {
      toastManager.add({
        title: `Se importaron ${scanResult.playlist_count} lista(s) M3U a tus playlists.`,
        type: 'success',
        timeout: 5000,
      });
    }
  } catch (err) {
    logAndNotifyError(err);
  } finally {
    useLibraryStore.setState({
      refreshing: false,
      refresh: { current: 0, total: 0 },
    });
  }
}

async function addLibraryFolders(paths: string | Array<string>): Promise<void> {
  try {
    const incoming = (Array.isArray(paths) ? paths : [paths]).filter(
      (p): p is string => typeof p === 'string' && p.trim().length > 0,
    );
    if (incoming.length === 0) {
      return;
    }
    const musicFolders = await ConfigBridge.get('library_folders');
    const newFolders = removeRedundantFolders([
      ...musicFolders,
      ...incoming,
    ]).sort();
    await ConfigBridge.set('library_folders', newFolders);
    await invalidateOmkfCatalogQueries();
  } catch (err) {
    logAndNotifyError(err);
  }
}

async function importAudioFiles(
  filePaths: string | Array<string>,
): Promise<number> {
  const normalized = (
    Array.isArray(filePaths) ? filePaths : [filePaths]
  ).filter((p): p is string => typeof p === 'string' && p.trim().length > 0);
  if (normalized.length === 0) {
    return 0;
  }

  try {
    useLibraryStore.setState({ refreshing: true });
    const parentDirs = Array.from(
      new Set(
        normalized
          .map(extractParentDirectory)
          .filter((dir): dir is string => Boolean(dir)),
      ),
    );
    if (parentDirs.length > 0) {
      await addLibraryFolders(parentDirs);
    }

    const scanResult = await DatabaseBridge.importTracks(normalized, false);
    if (!isTauri()) {
      const mockTracks = await DatabaseBridge.getAllTracks();
      syncMockCatalogTracks(mockTracks);
      useLibraryStore.setState({ tracks: mockTracks });
    }
    await invalidateOmkfCatalogQueries();

    if (scanResult.track_count > 0) {
      toastManager.add({
        title: `Se importaron ${scanResult.track_count} pista(s) con BPM, llave Camelot y 4 Hot Cues.`,
        type: 'success',
        timeout: 5000,
      });
    } else {
      toastManager.add({
        title:
          scanResult.track_failures > 0
            ? 'No se pudieron leer los archivos de audio seleccionados.'
            : 'Las pistas seleccionadas ya están presentes en tus carpetas.',
        type: scanResult.track_failures > 0 ? 'warning' : 'info',
        timeout: 4500,
      });
    }

    return scanResult.track_count;
  } catch (err) {
    logAndNotifyError(err);
    return 0;
  } finally {
    useLibraryStore.setState({
      refreshing: false,
      refresh: { current: 0, total: 0 },
    });
  }
}

async function removeLibraryFolder(path: string): Promise<void> {
  const musicFolders = await ConfigBridge.get('library_folders');
  const updatedMusicFolders = musicFolders.filter(
    (folderPath: string) => folderPath !== path,
  );

  if (updatedMusicFolders.length === musicFolders.length) {
    return;
  }

  await ConfigBridge.set('library_folders', updatedMusicFolders);
  await invalidateOmkfCatalogQueries();
}

function setRefresh(current: number, total: number): void {
  useLibraryStore.setState({ refresh: { current, total } });
}

/**
 * Remove tracks from legacy SQLite views
 */
async function removeTracks(tracksIDs: string[]): Promise<void> {
  await DatabaseBridge.removeTracks(tracksIDs);
}

/**
 * Reset the library
 */
async function reset(): Promise<void> {
  player.stop();
  try {
    await DatabaseBridge.reset();
    await ConfigBridge.set('library_folders', []);
    syncMockCatalogTracks([]);
    useLibraryStore.setState({ tracks: [], setSpineOrderedIds: null });
    await invalidateOmkfCatalogQueries();
    toastManager.add({
      title: 'Carpetas restablecidas correctamente.',
      type: 'success',
    });
  } catch (err) {
    logAndNotifyError(err);
  }
}

/**
 * Update the ID3 attributes of a track.
 */
async function updateTrackMetadata(
  trackID: string,
  newFields: TrackMutation,
): Promise<void> {
  try {
    let [track] = await DatabaseBridge.getTracks([trackID]);

    if (!track) {
      throw new Error('No track found while trying to update track metadata');
    }

    track = { ...track, ...newFields };

    await DatabaseBridge.updateTrack(track);
  } catch (err) {
    logAndNotifyError(err, 'Something wrong happened when updating the track');
  }
}

/**
 * Manually set the footer content based on a list of tracks
 */
function setTracksStatus(status?: TrackListStatusInfo): void {
  useLibraryStore.setState({ tracksStatus: status ?? null });
}

const LibraryAPI = {
  search,
  scan,
  addLibraryFolders,
  importAudioFiles,
  removeLibraryFolder,
  setRefresh,
  removeTracks,
  reset,
  updateTrackMetadata,
  setTracksStatus,
};

export default LibraryAPI;

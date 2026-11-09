import ConfigBridge from '../lib/bridge-config';
import DatabaseBridge from '../lib/bridge-database';
import player from '../lib/player';
import queryClient from '../lib/query-client';
import useLibraryStore from '../lib/store';
import toastManager from '../lib/toast-manager';
import { logAndNotifyError } from '../lib/utils';
import { removeRedundantFolders } from '../lib/utils-library';
import type { TrackListStatusInfo, TrackMutation } from '../types/museeks';

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
async function scan(
  // Force a refresh of the ID3 tags stored in the DB
  refresh = false,
): Promise<void> {
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
    const allTracks = await DatabaseBridge.getAllTracks();
    useLibraryStore.setState({ tracks: allTracks });
    await queryClient.invalidateQueries();

    if (scanResult.track_count > 0) {
      const message = refresh
        ? `Se actualizaron ${scanResult.track_count} pista(s) con análisis DSP y 4 Hot Cues.`
        : `Se importaron ${scanResult.track_count} pista(s) a tu biblioteca de cabina.`;

      toastManager.add({ title: message, type: 'success', timeout: 5000 });
    } else if (scanResult.playlist_count === 0) {
      toastManager.add({
        title:
          allTracks.length === 0
            ? 'No se encontraron archivos de audio (MP3, WAV, FLAC, AIFF, M4A, OGG) en la carpeta seleccionada.'
            : `Biblioteca al día (${allTracks.length} pista(s) listas en cabina).`,
        type: allTracks.length === 0 ? 'warning' : 'info',
        timeout: 5000,
      });
    }

    if (scanResult.playlist_count > 0) {
      toastManager.add({
        title: `Se importaron ${scanResult.playlist_count} lista(s) M3U a tu biblioteca.`,
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
    const allTracks = await DatabaseBridge.getAllTracks();
    useLibraryStore.setState({ tracks: allTracks });
    await queryClient.invalidateQueries();

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
            : 'Las pistas seleccionadas ya están presentes en tu biblioteca.',
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
    // Path to remove not found: no changes to persist
    return;
  }

  await ConfigBridge.set('library_folders', updatedMusicFolders);
}

function setRefresh(current: number, total: number): void {
  useLibraryStore.setState({ refresh: { current, total } });
}

/**
 * Remove tracks from the library
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
    useLibraryStore.setState({ tracks: [], setSpineOrderedIds: null });
    await queryClient.invalidateQueries();
    toastManager.add({
      title: 'Biblioteca restablecida correctamente.',
      type: 'success',
    });
  } catch (err) {
    logAndNotifyError(err);
  }
}

/**
 * Update the ID3 attributes of a track.
 * IMPROVE ME: add support for writing metadata to disk (and not only update
 * the DB).
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

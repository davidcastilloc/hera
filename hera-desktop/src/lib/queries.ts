import ConfigBridge from './bridge-config';
import DatabaseBridge from './bridge-database';
import useLibraryStore from './store';

export const allTracksQuery = {
  queryKey: ['all_tracks'],
  queryFn: async () => {
    try {
      const tracks = await DatabaseBridge.getAllTracks();
      useLibraryStore.setState({ tracks });
      return tracks;
    } catch {
      useLibraryStore.setState({ tracks: [] });
      return [];
    }
  },
  staleTime: Infinity,
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
};

export const configQuery = {
  queryKey: ['config'],
  queryFn: () => ConfigBridge.getAll(),
  initialData:
    typeof window !== 'undefined' ? window.__MUSEEKS_INITIAL_CONFIG : undefined,
  staleTime: Infinity,
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
};

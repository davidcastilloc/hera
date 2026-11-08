import ConfigBridge from './bridge-config';
import DatabaseBridge from './bridge-database';
import useLibraryStore from './store';

export const allTracksQuery = {
  queryKey: ['all_tracks'],
  queryFn: async () => {
    const tracks = await DatabaseBridge.getAllTracks();
    useLibraryStore.setState({ tracks });
    return tracks;
  },
  staleTime: Infinity,
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
};

export const configQuery = {
  queryKey: ['config'],
  queryFn: () => ConfigBridge.getAll(),
  initialData: window.__MUSEEKS_INITIAL_CONFIG,
  staleTime: Infinity,
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
};

import { create } from 'zustand';

import type { Track } from '../generated/typings';
import type { TrackListStatusInfo } from '../types/museeks';

type LibraryState = {
  tracks: Array<Track>;
  setSpineOrderedIds: Array<string> | null;
  search: string;
  refreshing: boolean;
  refresh: {
    current: number;
    total: number;
  };
  tracksStatus: TrackListStatusInfo | null;
};

const useLibraryStore = create<LibraryState>()(() => ({
  tracks: [],
  setSpineOrderedIds: null,
  search: '',
  refreshing: false,
  refresh: {
    current: 0,
    total: 0,
  },
  tracksStatus: null,
}));

export default useLibraryStore;

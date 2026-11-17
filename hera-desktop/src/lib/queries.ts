import ConfigBridge from './bridge-config';

export const configQuery = {
  queryKey: ['config'],
  queryFn: () => ConfigBridge.getAll(),
  initialData:
    typeof window !== 'undefined' ? window.__MUSEEKS_INITIAL_CONFIG : undefined,
  staleTime: Infinity,
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
};

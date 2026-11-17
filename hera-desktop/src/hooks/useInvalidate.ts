import { useQueryClient } from '@tanstack/react-query';
import { useRouter } from '@tanstack/react-router';
import { useCallback } from 'react';

import { configQuery } from '../lib/queries';
import { logAndNotifyError } from '../lib/utils';

/**
 * Hook returning a function to be manually called after anything that represents
 * a mutation. Invalidates OMKF folder crates, playlists, and config without
 * triggering full SQLite track table dumps.
 */
export default function useInvalidate() {
  const queryClient = useQueryClient();
  const router = useRouter();

  return useCallback(async () => {
    return Promise.allSettled([
      queryClient.invalidateQueries({ queryKey: ['omkf-folder-crates'] }),
      queryClient.invalidateQueries({ queryKey: ['omkf-folder-crate-detail'] }),
      queryClient.invalidateQueries({ queryKey: ['omkf-global-search'] }),
      queryClient.invalidateQueries({ queryKey: ['playlists'] }),
      queryClient.invalidateQueries({ queryKey: configQuery.queryKey }),
      router.invalidate(),
    ]);
  }, [queryClient, router]);
}

type AnyArgs = any[];
type Callback = (...args: AnyArgs) => Promise<void>;

/**
 * Helper hook to easily-wrap async mutations to reload routes data once its
 * callback is executed.
 */
export function useInvalidateCallback<T extends Callback>(
  callback: T,
): (...args: Parameters<T>) => Promise<void> {
  const invalidate = useInvalidate();

  return useCallback(
    async (...args: Parameters<T>) => {
      await callback(...args)
        .catch(logAndNotifyError)
        .finally(invalidate);
    },
    [callback, invalidate],
  );
}

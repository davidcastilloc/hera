import { useSyncExternalStore } from 'react';

import player, { type PlayerState as PlayerStateType } from '../lib/player';

/**
 * Hook to get a specific slice of player state.
 * Uses useSyncExternalStore for optimal performance and only re-renders when the selected value changes.
 */
export function usePlayerState<T>(selector: (state: PlayerStateType) => T): T {
  return useSyncExternalStore(
    (callback) => {
      player.on('stateChange', callback);
      return () => {
        player.off('stateChange', callback);
      };
    },
    () => selector(player.getState()),
    () => selector(player.getState()),
  );
}

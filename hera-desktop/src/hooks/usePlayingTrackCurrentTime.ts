import { useEffect, useRef, useState } from 'react';

import player from '../lib/player';

/**
 * Returns the current track elapsed time, throttled by `minDeltaSec` to curb re-renders.
 */
export default function usePlayingTrackCurrentTime(
  minDeltaSec = 0.033,
): number {
  const [currentTime, setCurrentTime] = useState(player.getCurrentTime());
  const lastRef = useRef(currentTime);

  useEffect(() => {
    function tick(time: number) {
      if (time === 0 || Math.abs(time - lastRef.current) >= minDeltaSec) {
        lastRef.current = time;
        setCurrentTime(time);
      }
    }

    player.on('timeupdate', tick);

    return () => {
      player.off('timeupdate', tick);
    };
  }, [minDeltaSec]);

  return currentTime;
}

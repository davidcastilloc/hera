import * as stylex from '@stylexjs/stylex';
import { useMemo, useState } from 'react';

import { usePlayerState } from '../hooks/usePlayer';
import player from '../lib/player';
import Icon from './Icon';

type Props = {
  isBufferingOverride?: boolean;
};

export default function TrackPlayingIndicator(props: Props) {
  const [hovered, setHovered] = useState(false);
  const isPaused = usePlayerState((state) => state.isPaused);
  const isLoading = usePlayerState(
    (state) => state.isLoadingTrack || state.isBuffering,
  );
  const showLoading = Boolean(props.isBufferingOverride) || isLoading;

  const icon = useMemo(() => {
    if (showLoading) {
      return (
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="#14b8a6"
          strokeWidth="2.2"
          strokeLinecap="round"
          aria-hidden="true"
          {...stylex.props(styles.spinner)}
        >
          <path d="M12 3a9 9 0 1 0 9 9" />
        </svg>
      );
    }

    if (!isPaused) {
      if (hovered) {
        return <Icon name="pause" size={12} />;
      }

      return (
        <div {...stylex.props(styles.animation)}>
          <div {...stylex.props(styles.bar)} />
          <div {...stylex.props(styles.bar, styles.barSecond)} />
          <div {...stylex.props(styles.bar, styles.barThird)} />
        </div>
      );
    }

    return <Icon name="play" />;
  }, [showLoading, isPaused, hovered]);

  return (
    <button
      type="button"
      onClick={() => player.playPause()}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      tabIndex={0}
      aria-label={
        showLoading ? 'Cargando pista' : isPaused ? 'Reproducir' : 'Pausar'
      }
      data-museeks-action
      {...stylex.props(styles.playingIndicator)}
    >
      {icon}
    </button>
  );
}

const barAnimation = stylex.keyframes({
  '0%': { transform: 'scale3d(1, 0, 1)' },
  '50%': { transform: 'scale3d(1, 1, 1)' },
  '100%': { transform: 'scale3d(1, 0, 1)' },
});

const spinAnimation = stylex.keyframes({
  '0%': { transform: 'rotate(0deg)' },
  '100%': { transform: 'rotate(360deg)' },
});

const styles = stylex.create({
  playingIndicator: {
    height: '100%',
    width: '100%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    borderStyle: 'none',
    backgroundColor: 'transparent',
    padding: 0,
  },
  spinner: {
    animationName: spinAnimation,
    animationDuration: '0.75s',
    animationTimingFunction: 'linear',
    animationIterationCount: 'infinite',
  },
  animation: {
    width: '8px',
    height: '8px',
    display: 'flex',
    alignItems: 'flex-end',
    justifyContent: 'space-between',
  },
  bar: {
    height: '8px',
    width: '2px',
    alignItems: 'baseline',
    backgroundColor: 'currentcolor',
    animationName: barAnimation,
    animationDuration: '1s',
    animationTimingFunction: 'ease-in-out',
    animationIterationCount: 'infinite',
    transformOrigin: 'bottom',
  },
  barSecond: {
    animationDelay: '0.55s',
  },
  barThird: {
    animationDelay: '0.25s',
  },
});

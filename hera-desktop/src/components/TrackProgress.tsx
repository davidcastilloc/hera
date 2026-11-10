import { Slider } from '@base-ui/react/slider';
import * as stylex from '@stylexjs/stylex';
import { useCallback, useMemo, useState } from 'react';

import type { Track } from '../generated/typings';
import useFormattedDuration from '../hooks/useFormattedDuration';
import usePlayingTrackCurrentTime from '../hooks/usePlayingTrackCurrentTime';
import { getFourHotCuesForTrack } from '../lib/bridge-hera';
import player from '../lib/player';
import LiveSpectrumCanvas from './LiveSpectrumCanvas';

type Props = {
  trackPlaying: Track;
};

export default function TrackProgress(props: Props) {
  const { trackPlaying } = props;

  const elapsed = usePlayingTrackCurrentTime();
  const hotCues = useMemo(
    () => getFourHotCuesForTrack(trackPlaying),
    [trackPlaying],
  );

  const jumpAudioTo = useCallback((value: number) => {
    player.setCurrentTime(value);
  }, []);

  const [tooltipTargetTime, setTooltipTargetTime] = useState<null | number>(
    null,
  );
  const [tooltipX, setTooltipX] = useState<null | number>(null);

  const showTooltip = useCallback(
    (e: React.MouseEvent<HTMLElement>) => {
      const { offsetX } = e.nativeEvent;
      const barWidth = e.currentTarget.offsetWidth;

      const percent = (offsetX / barWidth) * 100;
      const time = (percent * trackPlaying.duration) / 100;

      setTooltipTargetTime(time);
      setTooltipX(percent);
    },
    [trackPlaying],
  );

  const hideTooltip = useCallback(() => {
    setTooltipTargetTime(null);
    setTooltipX(null);
  }, []);

  const tooltipContent = useFormattedDuration(tooltipTargetTime);

  return (
    <Slider.Root
      min={0}
      max={trackPlaying.duration}
      step={1}
      value={elapsed}
      onValueChange={jumpAudioTo}
    >
      <Slider.Control
        {...stylex.props(styles.trackRoot)}
        onMouseMoveCapture={showTooltip}
        onMouseLeave={hideTooltip}
      >
        <Slider.Track {...stylex.props(styles.trackProgress)}>
          <LiveSpectrumCanvas />
          <Slider.Indicator {...stylex.props(styles.trackRange)} />
          {hotCues.map((cue) => {
            const pct = Math.min(
              99,
              Math.max(
                0.5,
                (cue.seconds / Math.max(1, trackPlaying.duration)) * 100,
              ),
            );
            return (
              <button
                key={cue.num}
                type="button"
                title={`${cue.name} (${cue.seconds.toFixed(1)}s)`}
                onClick={(ev) => {
                  ev.stopPropagation();
                  jumpAudioTo(cue.seconds);
                }}
                {...stylex.props(styles.hotCuePin)}
                style={{
                  left: `${pct}%`,
                  backgroundColor: cue.color,
                }}
              />
            );
          })}
          <div
            {...stylex.props(styles.progressTooltip)}
            style={{
              left: `${tooltipX}%`,
              display: tooltipX == null ? 'none' : 'block',
            }}
          >
            {tooltipContent}
          </div>
        </Slider.Track>
        <Slider.Thumb />
      </Slider.Control>
    </Slider.Root>
  );
}

const styles = stylex.create({
  trackRoot: {
    position: 'relative',
    display: 'flex',
    alignItems: 'center',
    userSelect: 'none',
    touchAction: 'none',
    height: '8px',
    transform: 'translateY(4px)',
    outline: {
      ':has(:focus-visible)': '1.5px solid var(--focus-color)',
    },
  },
  trackProgress: {
    display: 'block',
    position: 'relative',
    width: '100%',
    height: '100%',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '4px',
  },
  trackRange: {
    position: 'absolute',
    height: '100%',
    backgroundColor: '#14b8a6',
    borderRadius: '3px',
  },
  hotCuePin: {
    position: 'absolute',
    top: '-3px',
    width: '6px',
    height: '12px',
    padding: 0,
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#0c0e12',
    borderRadius: '2px',
    cursor: 'pointer',
    zIndex: 3,
    transform: 'translateX(-3px)',
  },
  progressTooltip: {
    position: 'absolute',
    backgroundColor: '#171c24',
    color: '#edeff3',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '4px',
    fontSize: '10.5px',
    fontVariantNumeric: 'tabular-nums',
    paddingBlock: '2px',
    paddingInline: '6px',
    bottom: '12px',
    zIndex: 1,
    transform: 'translateX(-50%)',
    pointerEvents: 'none',
  },
});

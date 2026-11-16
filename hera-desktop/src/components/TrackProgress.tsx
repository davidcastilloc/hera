import { Slider } from '@base-ui/react/slider';
import * as stylex from '@stylexjs/stylex';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type { Track } from '../generated/typings';
import useFormattedDuration from '../hooks/useFormattedDuration';
import { usePlayerState } from '../hooks/usePlayer';
import { getFourHotCuesForTrack } from '../lib/bridge-hera';
import player from '../lib/player';
import LiveSpectrumCanvas from './LiveSpectrumCanvas';
import { WaveformPeaksCanvas } from './WaveformPeaksCanvas';

type Props = {
  trackPlaying: Track;
};

export default function TrackProgress(props: Props) {
  const { trackPlaying } = props;
  const isPlaying = usePlayerState((state) => !state.isPaused);
  const [elapsed, setElapsed] = useState(() => player.getCurrentTime());

  useEffect(() => {
    setElapsed(player.getCurrentTime());
    const onTimeUpdate = (t: number) => setElapsed(t);
    player.on('timeupdate', onTimeUpdate);
    return () => {
      player.off('timeupdate', onTimeUpdate);
    };
  }, [trackPlaying.id]);

  useEffect(() => {
    if (!isPlaying) return;
    let rafId = 0;
    const step = () => {
      setElapsed(player.getCurrentTime());
      rafId = requestAnimationFrame(step);
    };
    rafId = requestAnimationFrame(step);
    return () => cancelAnimationFrame(rafId);
  }, [isPlaying]);

  const hotCues = useMemo(
    () => getFourHotCuesForTrack(trackPlaying),
    [trackPlaying],
  );
  const jumpAudioTo = useCallback(
    (value: number) => player.setCurrentTime(value),
    [],
  );

  const [tooltipTargetTime, setTooltipTargetTime] = useState<null | number>(
    null,
  );
  const [tooltipX, setTooltipX] = useState<null | number>(null);

  const showTooltip = useCallback(
    (e: React.MouseEvent<HTMLElement>) => {
      const { offsetX } = e.nativeEvent;
      const barWidth = Math.max(1, e.currentTarget.offsetWidth);
      const percent = (offsetX / barWidth) * 100;
      const time = (percent * Math.max(1, trackPlaying.duration)) / 100;
      setTooltipTargetTime(time);
      setTooltipX(percent);
    },
    [trackPlaying.duration],
  );

  const hideTooltip = useCallback(() => {
    setTooltipTargetTime(null);
    setTooltipX(null);
  }, []);

  const tooltipContent = useFormattedDuration(tooltipTargetTime);
  const durationMax = Math.max(1, trackPlaying.duration);

  return (
    <Slider.Root
      min={0}
      max={durationMax}
      step={0.01}
      value={Math.min(elapsed, durationMax)}
      onValueChange={jumpAudioTo}
    >
      <Slider.Control
        {...stylex.props(styles.trackRoot)}
        onMouseMoveCapture={showTooltip}
        onMouseLeave={hideTooltip}
      >
        <Slider.Track {...stylex.props(styles.trackProgress)}>
          <WaveformPeaksCanvas
            peaks={trackPlaying.waveform_peaks_blob}
            peaksB64={trackPlaying.waveform_peaks_b64}
            progressPct={(elapsed / durationMax) * 100}
          />
          <LiveSpectrumCanvas />
          <Slider.Indicator {...stylex.props(styles.trackRange)} />
          {hotCues.map((cue) => {
            const pct = Math.min(
              99,
              Math.max(0.5, (cue.seconds / durationMax) * 100),
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
                style={{ left: `${pct}%`, backgroundColor: cue.color }}
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
    outline: { ':has(:focus-visible)': '1.5px solid var(--focus-color)' },
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

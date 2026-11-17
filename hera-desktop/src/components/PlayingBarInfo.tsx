import * as stylex from '@stylexjs/stylex';

import type { Repeat, Track } from '../generated/typings';
import useFormattedDuration from '../hooks/useFormattedDuration';
import { usePlayerState } from '../hooks/usePlayer';
import usePlayingTrackCurrentTime from '../hooks/usePlayingTrackCurrentTime';
import TrackProgress from './TrackProgress';

type Props = {
  trackPlaying: Track;
  shuffle: boolean;
  repeat: Repeat;
};

export default function PlayingBarInfo(props: Props) {
  const { trackPlaying } = props;
  const elapsed = usePlayingTrackCurrentTime(0.5);
  const isWaitingAudio = usePlayerState(
    (state) => state.isLoadingTrack || state.isBuffering,
  );
  const formattedDuration = useFormattedDuration(trackPlaying.duration);
  const formattedProgress = useFormattedDuration(
    Math.min(trackPlaying.duration, elapsed),
  );

  return (
    <div {...stylex.props(styles.playingBarInfo)} data-tauri-drag-region>
      <div {...stylex.props(styles.playingBarInfoMetas)}>
        <div {...stylex.props(styles.duration)}>{formattedProgress}</div>

        <div {...stylex.props(styles.metas)}>
          <strong {...stylex.props(styles.metadata, styles.metadataTitle)}>
            {trackPlaying.title}
            {isWaitingAudio && (
              <span {...stylex.props(styles.bufferingPill)}>
                Cargando pista…
              </span>
            )}
          </strong>
          <div {...stylex.props(styles.metadata, styles.metadataSecondary)}>
            <span>{trackPlaying.artists.join(', ')}</span>
            &nbsp;—&nbsp;
            <span>{trackPlaying.album}</span>
          </div>
        </div>

        <div {...stylex.props(styles.duration)}>{formattedDuration}</div>
      </div>
      <TrackProgress trackPlaying={trackPlaying} />
    </div>
  );
}

const styles = stylex.create({
  playingBarInfo: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: 'auto',
    minWidth: 0,
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'flex-end',
    paddingTop: 0,
    paddingBottom: 0,
    paddingLeft: '8px',
    paddingRight: '8px',
  },
  playingBarInfoMetas: {
    rowGap: '4px',
    columnGap: '4px',
    display: 'flex',
    alignItems: 'flex-end',
    justifyContent: 'center',
    pointerEvents: 'none',
  },
  metas: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: 'auto',
    whiteSpace: 'nowrap',
    textOverflow: 'ellipsis',
    overflow: 'hidden',
    verticalAlign: 'middle',
    textAlign: 'center',
  },
  metadataTitle: {
    fontWeight: 'var(--bold)',
    marginBottom: '2px',
  },
  bufferingPill: {
    display: 'inline-block',
    marginLeft: '8px',
    paddingBlock: '1px',
    paddingInline: '6px',
    borderRadius: '4px',
    fontSize: '10.5px',
    fontWeight: 600,
    fontVariantNumeric: 'tabular-nums',
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    verticalAlign: 'middle',
  },
  metadata: {
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  metadataSecondary: {
    color: '#8e98a8',
    fontSize: '12px',
  },
  duration: {
    flexGrow: 0,
    flexShrink: 0,
    flexBasis: 'auto',
    fontSize: '0.875rem',
    paddingTop: 0,
    paddingBottom: 0,
    paddingLeft: '2px',
    paddingRight: '2px',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
    opacity: 0.7,
  },
});

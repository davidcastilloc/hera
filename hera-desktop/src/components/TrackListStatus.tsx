import * as stylex from '@stylexjs/stylex';

import useFormattedDuration from '../hooks/useFormattedDuration';
import type { TrackListStatusInfo } from '../types/museeks';

export default function TrackListStatus(props: TrackListStatusInfo) {
  const duration = useFormattedDuration(props.duration);
  const trackLabel = props.count === 1 ? '1 pista' : `${props.count} pistas`;

  return (
    <span {...stylex.props(styles.readout)}>
      {trackLabel} · {duration}
    </span>
  );
}

const styles = stylex.create({
  readout: {
    fontVariantNumeric: 'tabular-nums',
    color: 'var(--text-muted)',
  },
});

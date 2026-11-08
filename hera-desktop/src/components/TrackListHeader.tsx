import { useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';

import TrackListHeaderCell from './TrackListHeaderCell';

type Props = {
  sortable?: boolean;
};

export default function TrackListHeader({ sortable = true }: Props) {
  const { t } = useLingui();

  return (
    <div
      {...stylex.props(styles.trackListHeader)}
      role="group"
      aria-label={t`Track list sorting options`}
    >
      <TrackListHeaderCell
        xstyle={styles.cellTrackPlaying}
        title="&nbsp;"
        sortBy={null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellTrack}
        title={t`Title`}
        sortBy={sortable ? 'Title' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellBpm}
        title="BPM"
        sortBy={sortable ? 'Bpm' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellCamelot}
        title="Key"
        sortBy={sortable ? 'Camelot' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellCues}
        title="4 Hot Cues"
        sortBy={null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellDuration}
        title={t`Duration`}
        sortBy={sortable ? 'Duration' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellArtist}
        title={t`Artist`}
        sortBy={sortable ? 'Artist' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellAlbum}
        title={t`Album`}
        sortBy={sortable ? 'Album' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellGenre}
        title={t`Genre`}
        sortBy={sortable ? 'Genre' : null}
      />
    </div>
  );
}

const styles = stylex.create({
  trackListHeader: {
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: 'var(--border-color)',
    color: 'var(--tracks-header-color)',
    backgroundColor: 'var(--tracks-header-bg)',
    display: 'flex',
    width: '100%',
    position: 'sticky',
    top: 0,
    zIndex: 5,
  },
  cellTrackPlaying: {
    width: '30px',
    flexShrink: 0,
  },
  cellTrack: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
  },
  cellBpm: {
    width: '64px',
    minWidth: '64px',
    flexShrink: 0,
  },
  cellCamelot: {
    width: '56px',
    minWidth: '56px',
    flexShrink: 0,
  },
  cellCues: {
    width: '86px',
    minWidth: '86px',
    flexShrink: 0,
  },
  cellDuration: {
    width: '68px',
    minWidth: '68px',
    flexShrink: 0,
  },
  cellArtist: {
    width: '16%',
    flexShrink: 0,
  },
  cellAlbum: {
    width: '14%',
    flexShrink: 0,
  },
  cellGenre: {
    width: '12%',
    flexShrink: 0,
  },
});

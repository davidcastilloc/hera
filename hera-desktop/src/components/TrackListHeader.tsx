import * as stylex from '@stylexjs/stylex';

import TrackListHeaderCell from './TrackListHeaderCell';

type Props = {
  sortable?: boolean;
};

export default function TrackListHeader({ sortable = true }: Props) {
  return (
    <div
      {...stylex.props(styles.trackListHeader)}
      role="group"
      aria-label="Columnas y orden de pistas"
    >
      <TrackListHeaderCell
        xstyle={styles.cellTrackPlaying}
        title="&nbsp;"
        sortBy={null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellTrack}
        title="Título"
        sortBy={sortable ? 'Title' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellBpm}
        title="BPM"
        sortBy={sortable ? 'Bpm' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellCamelot}
        title="Tono"
        sortBy={sortable ? 'Camelot' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellCues}
        title="4 Cues"
        sortBy={null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellDuration}
        title="Duración"
        sortBy={sortable ? 'Duration' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellArtist}
        title="Artista"
        sortBy={sortable ? 'Artist' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellAlbum}
        title="Álbum"
        sortBy={sortable ? 'Album' : null}
      />
      <TrackListHeaderCell
        xstyle={styles.cellGenre}
        title="Género"
        sortBy={sortable ? 'Genre' : null}
      />
    </div>
  );
}

const styles = stylex.create({
  trackListHeader: {
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    color: '#8e98a8',
    backgroundColor: '#12161d',
    display: 'flex',
    width: '100%',
    position: 'sticky',
    top: 0,
    zIndex: 5,
    fontSize: '11.5px',
    fontWeight: 600,
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
    width: '66px',
    minWidth: '66px',
    flexShrink: 0,
  },
  cellCamelot: {
    width: '58px',
    minWidth: '58px',
    flexShrink: 0,
  },
  cellCues: {
    width: '104px',
    minWidth: '104px',
    flexShrink: 0,
  },
  cellDuration: {
    width: '70px',
    minWidth: '70px',
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

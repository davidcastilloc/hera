import * as stylex from '@stylexjs/stylex';

import type { Track } from '../../generated/typings';
import { CAMELOT_OPTIONS } from '../../lib/hera-harmonic-utils';
import Icon from '../Icon';

export type TransitionDeckCardProps = {
  label: 'Actual' | 'Destino';
  tracks: Array<Track>;
  selectedTrackId: string;
  artist: string;
  title: string;
  bpm: number;
  camelot: string;
  selectAriaLabel: string;
  bpmAriaLabel: string;
  keyAriaLabel: string;
  fallbackBpm: number;
  onSelectTrack: (trackId: string) => void;
  onBpmChange: (bpm: number) => void;
  onCamelotChange: (camelot: string) => void;
  onSwapDecks?: () => void;
};

export default function TransitionDeckCard({
  label,
  tracks,
  selectedTrackId,
  artist,
  title,
  bpm,
  camelot,
  selectAriaLabel,
  bpmAriaLabel,
  keyAriaLabel,
  fallbackBpm,
  onSelectTrack,
  onBpmChange,
  onCamelotChange,
  onSwapDecks,
}: TransitionDeckCardProps) {
  return (
    <div {...stylex.props(styles.endpointBox)}>
      <div {...stylex.props(styles.endpointHeader)}>
        <span {...stylex.props(styles.endpointLabel)}>{label}</span>
        {onSwapDecks ? (
          <button
            type="button"
            onClick={onSwapDecks}
            title="Intercambiar pistas"
            aria-label="Intercambiar pista actual y destino"
            {...stylex.props(styles.swapIconBtn)}
          >
            <Icon name="shuffle" size={14} />
          </button>
        ) : null}
      </div>

      {tracks.length > 0 && (
        <select
          value={selectedTrackId}
          onChange={(e) => onSelectTrack(e.target.value)}
          aria-label={selectAriaLabel}
          {...stylex.props(styles.librarySelect)}
        >
          <option value="">Elegir de biblioteca...</option>
          {tracks.map((t) => (
            <option key={t.id} value={t.id}>
              {(t.artists[0] || 'Artista') + ' — ' + t.title}
            </option>
          ))}
        </select>
      )}

      <div {...stylex.props(styles.endpointTrackPill)}>
        {artist} — {title}
      </div>

      <div {...stylex.props(styles.endpointMetaRow)}>
        <input
          type="number"
          min={60}
          max={200}
          step={1}
          aria-label={bpmAriaLabel}
          value={bpm}
          onChange={(e) => onBpmChange(Number(e.target.value) || fallbackBpm)}
          {...stylex.props(styles.inlineNumber)}
        />
        <span>BPM ·</span>
        <select
          aria-label={keyAriaLabel}
          value={camelot}
          onChange={(e) => onCamelotChange(e.target.value)}
          {...stylex.props(styles.inlineKeySelect)}
        >
          {CAMELOT_OPTIONS.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

const styles = stylex.create({
  endpointBox: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    paddingBlock: '14px',
    paddingInline: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
    minWidth: 0,
  },
  endpointHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '6px',
  },
  endpointLabel: {
    fontSize: '12.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  swapIconBtn: {
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    fontSize: '13px',
    cursor: 'pointer',
    paddingBlock: '2px',
    paddingInline: '4px',
  },
  librarySelect: {
    width: '100%',
    paddingBlock: '5px',
    paddingInline: '7px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
    fontSize: '11.5px',
  },
  endpointTrackPill: {
    paddingBlock: '6px',
    paddingInline: '10px',
    borderRadius: '6px',
    backgroundColor: '#1a202b',
    color: '#edeff3',
    fontSize: '12.5px',
    fontWeight: 500,
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  endpointMetaRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '5px',
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  inlineNumber: {
    width: '48px',
    paddingBlock: '2px',
    paddingInline: '5px',
    borderRadius: '4px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12px',
    fontVariantNumeric: 'tabular-nums',
  },
  inlineKeySelect: {
    paddingBlock: '2px',
    paddingInline: '5px',
    borderRadius: '4px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12px',
    fontVariantNumeric: 'tabular-nums',
  },
});

import * as stylex from '@stylexjs/stylex';

import type { Track } from '../../generated/typings';
import {
  CHECK_MINI_SVG,
  FOUR_CUE_DOTS,
  type MiniSuggestionItem,
  type MiniTrackSummary,
  PAUSE_SVG,
  PLAY_SVG,
} from './MiniIcons';

type SuggestionRowProps = {
  index: number;
  item: MiniSuggestionItem;
  isSent: boolean;
  isBusy: boolean;
  isPlaying: boolean;
  onPushVdj: (item: MiniSuggestionItem) => void;
  onToggleAudition: (item: MiniSuggestionItem) => void;
};

function MiniSuggestionRow({
  index,
  item,
  isSent,
  isBusy,
  isPlaying,
  onPushVdj,
  onToggleAudition,
}: SuggestionRowProps) {
  return (
    <div {...stylex.props(styles.suggestionCard)}>
      <span {...stylex.props(styles.rowIndex)}>
        {String(index + 1).padStart(2, '0')}
      </span>

      <div {...stylex.props(styles.rowInfo)}>
        <div {...stylex.props(styles.rowTitle)}>
          {item.artist} — {item.title}
        </div>
        <div {...stylex.props(styles.rowSub)}>
          <span {...stylex.props(styles.rowMetaTabular)}>
            {Math.round(item.bpm)} · {item.camelot}
          </span>
          <button
            type="button"
            disabled={isBusy}
            onClick={() => onPushVdj(item)}
            {...stylex.props(
              styles.vdjInlineBtn,
              isSent && styles.vdjInlineBtnSent,
            )}
          >
            {isSent ? CHECK_MINI_SVG : null}
            <span>{isBusy ? '...' : isSent ? 'En VDJ' : '+ VDJ'}</span>
          </button>
        </div>
      </div>

      {FOUR_CUE_DOTS}

      <button
        type="button"
        aria-label={isPlaying ? 'Detener escucha' : 'Escuchar sugerencia'}
        onClick={() => onToggleAudition(item)}
        {...stylex.props(
          styles.squarePlayBtn,
          isPlaying && styles.squarePlayBtnActive,
        )}
      >
        {isPlaying ? PAUSE_SVG : PLAY_SVG}
      </button>
    </div>
  );
}

export type MiniSuggestionsTabProps = {
  tracks: Array<Track>;
  currentTrackInfo: MiniTrackSummary;
  selectedCurrentId: string;
  suggestions: Array<MiniSuggestionItem>;
  sentIds: Set<string>;
  busyId: string | null;
  auditionId: string | null;
  filterText: string;
  activePreset: string | null;
  onSelectCurrentId: (id: string) => void;
  onPushVdj: (item: MiniSuggestionItem) => void;
  onToggleAudition: (item: MiniSuggestionItem) => void;
  onFilterChange: (value: string) => void;
  onApplyPreset: (preset: 'suave' | 'energia' | 'sin_voz') => void;
};

export default function MiniSuggestionsTab({
  tracks,
  currentTrackInfo,
  selectedCurrentId,
  suggestions,
  sentIds,
  busyId,
  auditionId,
  filterText,
  activePreset,
  onSelectCurrentId,
  onPushVdj,
  onToggleAudition,
  onFilterChange,
  onApplyPreset,
}: MiniSuggestionsTabProps) {
  return (
    <div {...stylex.props(styles.cardBody)}>
      <div {...stylex.props(styles.actualStrip)}>
        <span {...stylex.props(styles.actualTag)}>ACTUAL</span>
        <span {...stylex.props(styles.actualDot)}>·</span>
        <span {...stylex.props(styles.actualTitle)}>
          {currentTrackInfo.artist} — {currentTrackInfo.title}
        </span>
        <span {...stylex.props(styles.actualMeta)}>
          {currentTrackInfo.bpm} · {currentTrackInfo.camelot}
        </span>

        {tracks.length > 0 ? (
          <select
            aria-label="Cambiar pista actual"
            value={selectedCurrentId}
            onChange={(e) => onSelectCurrentId(e.target.value)}
            {...stylex.props(styles.invisibleSelectOverlay)}
          >
            <option value="">Automático (VirtualDJ / Reproductor)</option>
            {tracks.map((t) => (
              <option key={t.id} value={t.id}>
                {(t.artists[0] || 'Artista') + ' — ' + t.title}
              </option>
            ))}
          </select>
        ) : null}
      </div>

      <div {...stylex.props(styles.suggestionsStack)}>
        {suggestions.map((item, idx) => (
          <MiniSuggestionRow
            key={item.id}
            index={idx}
            item={item}
            isSent={sentIds.has(item.id)}
            isBusy={busyId === item.id}
            isPlaying={auditionId === item.id}
            onPushVdj={onPushVdj}
            onToggleAudition={onToggleAudition}
          />
        ))}
      </div>

      <footer {...stylex.props(styles.cardFooter)}>
        <input
          type="text"
          value={filterText}
          onChange={(e) => onFilterChange(e.target.value)}
          placeholder="Ajustar..."
          aria-label="Ajustar sugerencias"
          {...stylex.props(styles.microInput)}
        />

        <div {...stylex.props(styles.presetGroup)}>
          <button
            type="button"
            onClick={() => onApplyPreset('suave')}
            {...stylex.props(
              styles.presetPill,
              activePreset === 'suave' && styles.presetPillActive,
            )}
          >
            Suave
          </button>
          <button
            type="button"
            onClick={() => onApplyPreset('energia')}
            {...stylex.props(
              styles.presetPill,
              activePreset === 'energia' && styles.presetPillActive,
            )}
          >
            + Energía
          </button>
          <button
            type="button"
            onClick={() => onApplyPreset('sin_voz')}
            {...stylex.props(
              styles.presetPill,
              activePreset === 'sin_voz' && styles.presetPillActive,
            )}
          >
            Sin voz
          </button>
        </div>
      </footer>
    </div>
  );
}

const styles = stylex.create({
  cardBody: {
    flexGrow: 1,
    minHeight: 0,
    padding: '10px',
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'space-between',
    rowGap: '8px',
  },
  actualStrip: {
    position: 'relative',
    height: '34px',
    flexShrink: 0,
    paddingInline: '10px',
    borderRadius: '7px',
    backgroundColor: '#141922',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    minWidth: 0,
  },
  actualTag: {
    fontSize: '10px',
    fontWeight: 600,
    letterSpacing: '0.04em',
    color: '#8e98a8',
    flexShrink: 0,
  },
  actualDot: {
    color: '#5c6675',
    flexShrink: 0,
  },
  actualTitle: {
    fontSize: '12px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
    flexGrow: 1,
  },
  actualMeta: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  invisibleSelectOverlay: {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    opacity: 0,
    cursor: 'pointer',
  },
  suggestionsStack: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    flexGrow: 1,
    justifyContent: 'center',
    minHeight: 0,
  },
  suggestionCard: {
    paddingBlock: '8px',
    paddingInline: '10px',
    borderRadius: '8px',
    backgroundColor: '#131820',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#202733',
    display: 'flex',
    alignItems: 'center',
    columnGap: '9px',
    minWidth: 0,
  },
  rowIndex: {
    fontSize: '11px',
    color: '#6d7787',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
    width: '16px',
  },
  rowInfo: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '4px',
    minWidth: 0,
    flexGrow: 1,
  },
  rowTitle: {
    fontSize: '12.5px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  rowSub: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  rowMetaTabular: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  vdjInlineBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    paddingBlock: '1px',
    paddingInline: '7px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#2a3342',
      ':hover': '#14b8a6',
    },
    backgroundColor: '#1a202b',
    color: '#edeff3',
    fontSize: '10.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  vdjInlineBtnSent: {
    borderColor: 'rgba(20, 184, 166, 0.45)',
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    color: '#2dd4bf',
  },
  squarePlayBtn: {
    width: '28px',
    height: '28px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#26303d',
      ':hover': '#14b8a6',
    },
    backgroundColor: '#171c24',
    color: '#edeff3',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
    cursor: 'pointer',
  },
  squarePlayBtnActive: {
    borderColor: '#14b8a6',
    backgroundColor: 'rgba(20, 184, 166, 0.16)',
    color: '#2dd4bf',
  },
  cardFooter: {
    paddingTop: '6px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1f2632',
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  microInput: {
    flexGrow: 1,
    minWidth: '82px',
    height: '28px',
    paddingInline: '9px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#26303d',
      ':focus': '#14b8a6',
    },
    backgroundColor: '#12161d',
    color: '#edeff3',
    fontSize: '11.5px',
    outline: 'none',
  },
  presetGroup: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    flexShrink: 0,
  },
  presetPill: {
    height: '26px',
    paddingInline: '8px',
    borderRadius: '999px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    backgroundColor: '#141922',
    color: '#cbd2dc',
    fontSize: '10.5px',
    fontWeight: 500,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  },
  presetPillActive: {
    borderColor: 'rgba(20, 184, 166, 0.5)',
    backgroundColor: 'rgba(20, 184, 166, 0.15)',
    color: '#2dd4bf',
  },
});

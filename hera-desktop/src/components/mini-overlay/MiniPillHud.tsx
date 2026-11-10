import * as stylex from '@stylexjs/stylex';

import {
  ARROW_RIGHT_SVG,
  CHECK_MINI_SVG,
  CHEVRON_UP_SVG,
  EXPAND_WINDOW_SVG,
  FOUR_CUE_DOTS,
  GRIP_DOTS_SVG,
  type MiniSuggestionItem,
  type MiniTrackSummary,
  PAUSE_SVG,
  PLAY_SVG,
} from './MiniIcons';

export type MiniPillHudProps = {
  currentTrackInfo: MiniTrackSummary;
  topSuggestion?: MiniSuggestionItem;
  isTopSent: boolean;
  isTopPlaying: boolean;
  hasBusyAction: boolean;
  onToggleAudition: (item: MiniSuggestionItem) => void;
  onPushVdj: (item: MiniSuggestionItem) => void;
  onExpandToCard: () => void;
  onRestoreFullWindow: () => void;
};

export default function MiniPillHud({
  currentTrackInfo,
  topSuggestion,
  isTopSent,
  isTopPlaying,
  hasBusyAction,
  onToggleAudition,
  onPushVdj,
  onExpandToCard,
  onRestoreFullWindow,
}: MiniPillHudProps) {
  return (
    <section
      aria-label="Barra Mini HERA"
      {...stylex.props(styles.pillRoot)}
      data-tauri-drag-region
    >
      <div {...stylex.props(styles.pillBrand)} data-tauri-drag-region>
        <span {...stylex.props(styles.gripHandle)} data-tauri-drag-region>
          {GRIP_DOTS_SVG}
        </span>
        <span {...stylex.props(styles.emeraldDot)} data-tauri-drag-region />
        <span {...stylex.props(styles.brandText)} data-tauri-drag-region>
          HERA
        </span>
      </div>

      <span {...stylex.props(styles.pillDivider)} data-tauri-drag-region />

      <div {...stylex.props(styles.pillNowPlaying)} data-tauri-drag-region>
        <span {...stylex.props(styles.pillMuted)} data-tauri-drag-region>
          Sonando
        </span>
        <span {...stylex.props(styles.pillTrackName)} data-tauri-drag-region>
          {currentTrackInfo.title}
        </span>
        <span {...stylex.props(styles.pillTabular)} data-tauri-drag-region>
          {currentTrackInfo.bpm}
        </span>
      </div>

      <span {...stylex.props(styles.pillArrow)} data-tauri-drag-region>
        {ARROW_RIGHT_SVG}
      </span>

      {topSuggestion ? (
        <div {...stylex.props(styles.pillNextBlock)} data-tauri-drag-region>
          <div {...stylex.props(styles.pillNextTop)} data-tauri-drag-region>
            <span
              {...stylex.props(styles.pillNextTitle)}
              data-tauri-drag-region
            >
              {topSuggestion.title}
            </span>
            <span {...stylex.props(styles.keyBadge)} data-tauri-drag-region>
              {topSuggestion.camelot}
            </span>
          </div>
          <div {...stylex.props(styles.pillNextSub)} data-tauri-drag-region>
            <span {...stylex.props(styles.pillTabular)}>
              {Math.round(topSuggestion.bpm)} BPM
            </span>
            {FOUR_CUE_DOTS}
          </div>
        </div>
      ) : null}

      <div {...stylex.props(styles.pillActions)}>
        {topSuggestion ? (
          <>
            <button
              type="button"
              aria-label={
                isTopPlaying ? 'Detener escucha' : 'Escuchar sugerencia'
              }
              onClick={() => onToggleAudition(topSuggestion)}
              {...stylex.props(
                styles.squarePlayBtn,
                isTopPlaying && styles.squarePlayBtnActive,
              )}
            >
              {isTopPlaying ? PAUSE_SVG : PLAY_SVG}
            </button>

            <button
              type="button"
              disabled={hasBusyAction}
              onClick={() => onPushVdj(topSuggestion)}
              {...stylex.props(
                styles.pillVdjBtn,
                isTopSent && styles.pillVdjBtnSent,
              )}
            >
              {isTopSent ? CHECK_MINI_SVG : null}
              <span>{isTopSent ? 'En VDJ' : '+ VDJ'}</span>
            </button>
          </>
        ) : null}

        <button
          type="button"
          aria-label="Desplegar tarjeta"
          title="Desplegar tarjeta"
          onClick={onExpandToCard}
          {...stylex.props(styles.headerIconBtn)}
        >
          {CHEVRON_UP_SVG}
        </button>

        <button
          type="button"
          aria-label="Volver a ventana completa"
          title="Volver a HERA completo"
          onClick={onRestoreFullWindow}
          {...stylex.props(styles.headerIconBtn)}
        >
          {EXPAND_WINDOW_SVG}
        </button>
      </div>
    </section>
  );
}

const styles = stylex.create({
  pillRoot: {
    boxSizing: 'border-box',
    width: '100%',
    height: '100%',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    borderRadius: '10px',
    paddingInline: '12px',
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    color: '#edeff3',
    overflow: 'hidden',
    userSelect: 'none',
  },
  pillBrand: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '7px',
    flexShrink: 0,
  },
  gripHandle: {
    display: 'inline-flex',
    alignItems: 'center',
    color: '#5c6675',
    cursor: 'grab',
  },
  emeraldDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  brandText: {
    fontSize: '13px',
    fontWeight: 700,
    letterSpacing: '0.03em',
    color: '#edeff3',
  },
  pillDivider: {
    width: '1px',
    height: '20px',
    backgroundColor: '#232a36',
    flexShrink: 0,
  },
  pillNowPlaying: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    minWidth: 0,
    maxWidth: '148px',
    flexShrink: 1,
  },
  pillMuted: {
    fontSize: '11px',
    color: '#8e98a8',
    flexShrink: 0,
  },
  pillTrackName: {
    fontSize: '12px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  pillTabular: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  pillArrow: {
    display: 'inline-flex',
    alignItems: 'center',
    color: '#5c6675',
    flexShrink: 0,
  },
  pillNextBlock: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    minWidth: 0,
    flexGrow: 1,
  },
  pillNextTop: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    minWidth: 0,
  },
  pillNextTitle: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  pillNextSub: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
  },
  keyBadge: {
    display: 'inline-flex',
    alignItems: 'center',
    paddingBlock: '1px',
    paddingInline: '5px',
    borderRadius: '4px',
    fontSize: '10px',
    fontWeight: 600,
    backgroundColor: 'rgba(20, 184, 166, 0.16)',
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  pillActions: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    flexShrink: 0,
  },
  pillVdjBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    height: '28px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#062320',
    fontSize: '11px',
    fontWeight: 600,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  },
  pillVdjBtnSent: {
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
  headerIconBtn: {
    width: '24px',
    height: '24px',
    borderRadius: '5px',
    borderWidth: 0,
    backgroundColor: {
      default: 'transparent',
      ':hover': '#171c24',
    },
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
    },
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
  },
});

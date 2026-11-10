import * as stylex from '@stylexjs/stylex';

import type { Track } from '../../generated/typings';
import type { HarmonicBridgeStep } from '../../lib/bridge-hera';
import {
  ARROW_RIGHT_SVG,
  CHECK_MINI_SVG,
  FOUR_CUE_DOTS,
  type MiniBridgeEndpoints,
  type MiniSuggestionItem,
  PAUSE_SVG,
  PLAY_SVG,
  SWAP_ARROWS_SVG,
} from './MiniIcons';

export type MiniBridgeTabProps = {
  tracks: Array<Track>;
  bridgeEndpoints: MiniBridgeEndpoints;
  bridgeOriginId: string;
  bridgeTargetId: string;
  bridgeSteps: Array<HarmonicBridgeStep>;
  auditionId: string | null;
  busyId: string | null;
  bridgeSent: boolean;
  onSelectOriginId: (id: string) => void;
  onSelectTargetId: (id: string) => void;
  onToggleAudition: (item: MiniSuggestionItem) => void;
  onPushBridgeToVdj: () => void;
  onSwapEndpoints: () => void;
};

export default function MiniBridgeTab({
  tracks,
  bridgeEndpoints,
  bridgeOriginId,
  bridgeTargetId,
  bridgeSteps,
  auditionId,
  busyId,
  bridgeSent,
  onSelectOriginId,
  onSelectTargetId,
  onToggleAudition,
  onPushBridgeToVdj,
  onSwapEndpoints,
}: MiniBridgeTabProps) {
  return (
    <div {...stylex.props(styles.cardBody)}>
      <div {...stylex.props(styles.bridgeEndpointsRow)}>
        <div {...stylex.props(styles.endpointMiniBox)}>
          <span {...stylex.props(styles.endpointMiniLabel)}>Actual</span>
          <span {...stylex.props(styles.endpointMiniTitle)}>
            {bridgeEndpoints.origin.artist} — {bridgeEndpoints.origin.title}
          </span>
          <span {...stylex.props(styles.endpointMiniMeta)}>
            {bridgeEndpoints.origin.bpm} · {bridgeEndpoints.origin.camelot}
          </span>

          {tracks.length > 0 ? (
            <select
              aria-label="Seleccionar origen"
              value={bridgeOriginId}
              onChange={(e) => onSelectOriginId(e.target.value)}
              {...stylex.props(styles.invisibleSelectOverlay)}
            >
              {tracks.map((t) => (
                <option key={t.id} value={t.id}>
                  {(t.artists[0] || 'Artista') + ' — ' + t.title}
                </option>
              ))}
            </select>
          ) : null}
        </div>

        <span {...stylex.props(styles.bridgeArrowEmerald)}>
          {ARROW_RIGHT_SVG}
        </span>

        <div {...stylex.props(styles.endpointMiniBox)}>
          <span {...stylex.props(styles.endpointMiniLabel)}>Destino</span>
          <span {...stylex.props(styles.endpointMiniTitle)}>
            {bridgeEndpoints.target.artist} — {bridgeEndpoints.target.title}
          </span>
          <span {...stylex.props(styles.endpointMiniMeta)}>
            {bridgeEndpoints.target.bpm} · {bridgeEndpoints.target.camelot}
          </span>

          {tracks.length > 0 ? (
            <select
              aria-label="Seleccionar destino"
              value={bridgeTargetId}
              onChange={(e) => onSelectTargetId(e.target.value)}
              {...stylex.props(styles.invisibleSelectOverlay)}
            >
              {tracks.map((t) => (
                <option key={t.id} value={t.id}>
                  {(t.artists[0] || 'Artista') + ' — ' + t.title}
                </option>
              ))}
            </select>
          ) : null}
        </div>
      </div>

      <div {...stylex.props(styles.suggestionsStack)}>
        {bridgeSteps.map((step, idx) => {
          const stepItem: MiniSuggestionItem = {
            id: step.track_id,
            artist: step.artist,
            title: step.title,
            bpm: step.bpm,
            camelot: step.camelot,
            cueSeconds: step.cue_in_seconds,
            heraSeal: step.hera_seal,
          };
          const isPlaying = auditionId === stepItem.id;
          return (
            <div key={step.track_id} {...stylex.props(styles.suggestionCard)}>
              <span {...stylex.props(styles.rowIndex)}>
                {String(idx + 1).padStart(2, '0')}
              </span>

              <div {...stylex.props(styles.rowInfo)}>
                <div {...stylex.props(styles.rowTitle)}>
                  {step.artist} — {step.title}
                </div>
                <div {...stylex.props(styles.rowMetaTabular)}>
                  {Math.round(step.bpm)} · {step.camelot}
                </div>
              </div>

              {FOUR_CUE_DOTS}

              <button
                type="button"
                aria-label={
                  isPlaying ? 'Detener escucha' : 'Escuchar paso del puente'
                }
                onClick={() => onToggleAudition(stepItem)}
                {...stylex.props(
                  styles.squarePlayBtn,
                  isPlaying && styles.squarePlayBtnActive,
                )}
              >
                {isPlaying ? PAUSE_SVG : PLAY_SVG}
              </button>
            </div>
          );
        })}
      </div>

      <footer {...stylex.props(styles.bridgeActionFooter)}>
        <button
          type="button"
          disabled={busyId === 'bridge-push'}
          onClick={onPushBridgeToVdj}
          {...stylex.props(styles.primaryBridgeBtn)}
        >
          {bridgeSent ? CHECK_MINI_SVG : null}
          <span>
            {busyId === 'bridge-push'
              ? 'Enviando...'
              : bridgeSent
                ? 'Puente en VirtualDJ'
                : 'Mandar puente a VirtualDJ'}
          </span>
        </button>

        <button
          type="button"
          aria-label="Intercambiar pistas"
          title="Intercambiar actual y destino"
          onClick={onSwapEndpoints}
          {...stylex.props(styles.swapSquareBtn)}
        >
          {SWAP_ARROWS_SVG}
        </button>
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
  bridgeEndpointsRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  endpointMiniBox: {
    position: 'relative',
    flexGrow: 1,
    flexBasis: '0%',
    minWidth: 0,
    paddingBlock: '7px',
    paddingInline: '9px',
    borderRadius: '7px',
    backgroundColor: '#141922',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
  },
  endpointMiniLabel: {
    fontSize: '10px',
    color: '#8e98a8',
  },
  endpointMiniTitle: {
    fontSize: '11.5px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  endpointMiniMeta: {
    fontSize: '10.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  invisibleSelectOverlay: {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    opacity: 0,
    cursor: 'pointer',
  },
  bridgeArrowEmerald: {
    color: '#2dd4bf',
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
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
  rowMetaTabular: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
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
  bridgeActionFooter: {
    paddingTop: '6px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1f2632',
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  primaryBridgeBtn: {
    flexGrow: 1,
    height: '32px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#062320',
    fontSize: '12px',
    fontWeight: 600,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    columnGap: '6px',
    cursor: 'pointer',
  },
  swapSquareBtn: {
    width: '32px',
    height: '32px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#26303d',
      ':hover': '#14b8a6',
    },
    backgroundColor: '#141922',
    color: {
      default: '#cbd2dc',
      ':hover': '#edeff3',
    },
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
    cursor: 'pointer',
  },
});

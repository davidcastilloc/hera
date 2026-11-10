import * as stylex from '@stylexjs/stylex';

import type { Track } from '../../generated/typings';
import type { HarmonicBridgeStep } from '../../lib/bridge-hera';
import { CUE_DOT_COLORS } from '../../lib/hera-harmonic-utils';
import Icon from '../Icon';

export function TransitionFlowArrow() {
  return (
    <div {...stylex.props(styles.connectorArrow)} aria-hidden="true">
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M5 12h14M13 6l6 6-6 6" />
      </svg>
    </div>
  );
}

export type TransitionBridgeStepsCardProps = {
  steps: Array<HarmonicBridgeStep>;
  activeAuditionId: string | null;
  findMatchingLocalTrack: (
    artist: string,
    title: string,
    trackId?: string,
  ) => Track | undefined;
  onAuditionStep: (
    auditionId: string,
    bpm: number,
    camelot: string,
    cueSeconds: number,
    localCandidate?: Track,
  ) => void;
};

export default function TransitionBridgeStepsCard({
  steps,
  activeAuditionId,
  findMatchingLocalTrack,
  onAuditionStep,
}: TransitionBridgeStepsCardProps) {
  return (
    <div {...stylex.props(styles.bridgeCenter)}>
      <h2 {...stylex.props(styles.bridgeHeading)}>Puente sugerido</h2>

      <div {...stylex.props(styles.stepsList)}>
        {steps.map((step, index) => {
          const auditionKey = `step-${step.track_id}-${index}`;
          const isPlaying = activeAuditionId === auditionKey;
          const localTrack = findMatchingLocalTrack(
            step.artist,
            step.title,
            step.track_id,
          );
          const rowIndex = String(index + 1).padStart(2, '0');

          return (
            <div
              key={auditionKey}
              {...stylex.props(
                styles.stepRow,
                index > 0 && styles.stepRowBorder,
              )}
            >
              <span {...stylex.props(styles.stepNum)}>{rowIndex}</span>
              <span {...stylex.props(styles.stepTitle)}>
                {step.artist} — {step.title}
              </span>
              <span {...stylex.props(styles.stepMeta)}>
                {Math.round(step.bpm)} BPM · {step.camelot}
              </span>
              <span
                {...stylex.props(styles.cueDots)}
                title="Puntos: Entrada · Calma · Clímax · Salida"
              >
                {CUE_DOT_COLORS.map((color) => (
                  <span
                    key={color}
                    {...stylex.props(styles.cueDot)}
                    style={{ backgroundColor: color }}
                  />
                ))}
              </span>
              <button
                type="button"
                onClick={() =>
                  onAuditionStep(
                    auditionKey,
                    step.bpm,
                    step.camelot,
                    step.cue_in_seconds,
                    localTrack,
                  )
                }
                {...stylex.props(
                  styles.listenBtn,
                  isPlaying && styles.listenBtnActive,
                )}
              >
                <Icon name={isPlaying ? 'pause' : 'play'} size={12} />
                <span>{isPlaying ? 'Detener' : 'Escuchar'}</span>
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}

const styles = stylex.create({
  connectorArrow: {
    textAlign: 'center',
    color: '#8e98a8',
    fontSize: '16px',
    userSelect: 'none',
  },
  bridgeCenter: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
    minWidth: 0,
  },
  bridgeHeading: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  stepsList: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    overflow: 'hidden',
  },
  stepRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
    paddingBlock: '12px',
    paddingInline: '14px',
    minWidth: 0,
  },
  stepRowBorder: {
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1e2530',
  },
  stepNum: {
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  stepTitle: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    fontSize: '13px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  stepMeta: {
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
    flexShrink: 0,
  },
  cueDots: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    flexShrink: 0,
  },
  cueDot: {
    width: '5.5px',
    height: '5.5px',
    borderRadius: '999px',
  },
  listenBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    backgroundColor: '#19202b',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
    flexShrink: 0,
  },
  listenBtnActive: {
    borderColor: '#14b8a6',
    color: '#2dd4bf',
  },
});

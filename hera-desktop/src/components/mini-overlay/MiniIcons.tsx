import * as stylex from '@stylexjs/stylex';

import type { Track } from '../../generated/typings';
import { CUE_DOT_COLORS } from '../../lib/hera-harmonic-utils';

export type MiniSuggestionItem = {
  id: string;
  artist: string;
  title: string;
  bpm: number;
  camelot: string;
  cueSeconds: number;
  heraSeal: string;
  localTrack?: Track;
};

export type MiniTrackSummary = {
  id: string;
  artist: string;
  title: string;
  bpm: number;
  camelot: string;
};

export type MiniBridgeEndpoints = {
  origin: MiniTrackSummary;
  target: MiniTrackSummary;
};

const styles = stylex.create({
  dots: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    flexShrink: 0,
  },
  dot: {
    width: '6px',
    height: '6px',
    borderRadius: '999px',
  },
});

export const GRIP_DOTS_SVG = (
  <svg
    width="10"
    height="14"
    viewBox="0 0 10 14"
    fill="currentColor"
    aria-hidden="true"
  >
    <circle cx="2.5" cy="2.5" r="1.1" />
    <circle cx="7.5" cy="2.5" r="1.1" />
    <circle cx="2.5" cy="7" r="1.1" />
    <circle cx="7.5" cy="7" r="1.1" />
    <circle cx="2.5" cy="11.5" r="1.1" />
    <circle cx="7.5" cy="11.5" r="1.1" />
  </svg>
);

export const ARROW_RIGHT_SVG = (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M5 12h14M13 6l6 6-6 6" />
  </svg>
);

export const PLAY_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
  >
    <path d="M7 5.5v13l11-6.5-11-6.5z" />
  </svg>
);

export const PAUSE_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
  >
    <rect x="6" y="5" width="4" height="14" rx="1" />
    <rect x="14" y="5" width="4" height="14" rx="1" />
  </svg>
);

export const CHECK_MINI_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  </svg>
);

export const PIN_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M15 4.5l4.5 4.5-4 1.5-3 3-1 4-6.5-6.5 4-1 3-3 1.5-4z" />
    <path d="M9 15l-4.5 4.5" />
  </svg>
);

export const CHEVRON_DOWN_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M6 9l6 6 6-6" />
  </svg>
);

export const CHEVRON_UP_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M18 15l-6-6-6 6" />
  </svg>
);

export const EXPAND_WINDOW_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M14 4h6v6M10 20H4v-6M20 4l-7 7M4 20l7-7" />
  </svg>
);

export const SPARKLE_MINI_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9L12 3z" />
  </svg>
);

export const SWAP_ARROWS_SVG = (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M17 3l4 4-4 4" />
    <path d="M3 7h18" />
    <path d="M7 21l-4-4 4-4" />
    <path d="M21 17H3" />
  </svg>
);

export const FOUR_CUE_DOTS = (
  <span
    {...stylex.props(styles.dots)}
    title="Entrada · Calma · Clímax · Salida"
  >
    {CUE_DOT_COLORS.map((color) => (
      <span
        key={color}
        {...stylex.props(styles.dot)}
        style={{ backgroundColor: color }}
      />
    ))}
  </span>
);

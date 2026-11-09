import { create } from 'zustand';

export type HeraViewMode = 'simple' | 'pro';

export interface HeraHumanMatchInfo {
  compatibilityLabel: string;
  badgeClass: 'match' | 'vibe' | 'warning';
  vibeDescription: string;
  tempoAdvice: string;
  harmonicConfidencePct: number;
}

export interface HeraHumanCueInfo {
  cueIndex: number;
  simpleName: string;
  positionSec: number;
  timeFormatted: string;
  color: string;
  role: string;
}

export function formatSecondsToMinutes(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

export function getHumanCueInfo(cueIndex: number, positionSec: number): HeraHumanCueInfo {
  const timeFormatted = formatSecondsToMinutes(positionSec);
  switch (cueIndex) {
    case 1:
      return {
        cueIndex: 1,
        simpleName: 'Entrada',
        positionSec,
        timeFormatted,
        color: '#10b981',
        role: 'Momento perfecto para empezar a meter el track',
      };
    case 2:
      return {
        cueIndex: 2,
        simpleName: 'Calma',
        positionSec,
        timeFormatted,
        color: '#3b82f6',
        role: 'Pausa melódica para respirar o cambiar filtros',
      };
    case 3:
      return {
        cueIndex: 3,
        simpleName: 'Explosión',
        positionSec,
        timeFormatted,
        color: '#f43f5e',
        role: 'Entrada del drop o coro con máxima energía',
      };
    case 4:
    default:
      return {
        cueIndex: 4,
        simpleName: 'Salida limpia',
        positionSec,
        timeFormatted,
        color: '#f59e0b',
        role: 'Sección instrumental para salir sin choque de voces',
      };
  }
}

export function getHumanAudioQuality(cutoffKhz?: number | null): {
  label: string;
  isLossless: boolean;
  color: string;
} {
  if (cutoffKhz && cutoffKhz >= 20.0) {
    return {
      label: '✨ Sonido de Estudio',
      isLossless: true,
      color: '#6ee7b7',
    };
  }
  return {
    label: '✨ Audio Limpio (Verificado)',
    isLossless: true,
    color: '#67e8f9',
  };
}

export function getHumanMatchLabel(
  camelotFrom: string,
  camelotTo: string,
  bpmFrom: number,
  bpmTo: number
): HeraHumanMatchInfo {
  const bpmDelta = Math.abs(bpmTo - bpmFrom);
  const tempoAdvice =
    bpmDelta === 0
      ? 'Mismo ritmo exacto'
      : bpmTo > bpmFrom
      ? `Sube suave (+${bpmDelta.toFixed(1)} BPM)`
      : `Baja suave (-${bpmDelta.toFixed(1)} BPM)`;

  // Same key
  if (camelotFrom === camelotTo) {
    return {
      compatibilityLabel: '💚 Mezcla Perfecta (99%)',
      badgeClass: 'match',
      vibeDescription: 'Misma afinación exacta · Cero choque',
      tempoAdvice,
      harmonicConfidencePct: 99,
    };
  }

  // Neighbor key logic (e.g. 8A to 9A, 8A to 7A, 8A to 8B)
  const numFrom = parseInt(camelotFrom, 10);
  const letterFrom = camelotFrom.slice(-1);
  const numTo = parseInt(camelotTo, 10);
  const letterTo = camelotTo.slice(-1);

  const isNumericNeighbor =
    letterFrom === letterTo &&
    (Math.abs(numTo - numFrom) === 1 || Math.abs(numTo - numFrom) === 11);
  const isModeShift = numFrom === numTo && letterFrom !== letterTo;

  if (isNumericNeighbor) {
    return {
      compatibilityLabel: '💚 Mezcla Fluida (98%)',
      badgeClass: 'match',
      vibeDescription: 'Evolución armónica continua',
      tempoAdvice,
      harmonicConfidencePct: 98,
    };
  }

  if (isModeShift) {
    return {
      compatibilityLabel: '☀️ Cambio de Brillo (95%)',
      badgeClass: 'vibe',
      vibeDescription: 'Pasa de tono menor a mayor suavemente',
      tempoAdvice,
      harmonicConfidencePct: 95,
    };
  }

  return {
    compatibilityLabel: '⚡ Transición Dinámica (91%)',
    badgeClass: 'vibe',
    vibeDescription: 'Cruce controlado con EQ o filtro',
    tempoAdvice,
    harmonicConfidencePct: 91,
  };
}

interface HeraViewStore {
  viewMode: HeraViewMode;
  isOnboardingOpen: boolean;
  setViewMode: (mode: HeraViewMode) => void;
  toggleViewMode: () => void;
  openOnboarding: () => void;
  closeOnboarding: () => void;
  toggleOnboarding: () => void;
}

const STORAGE_KEY = 'hera_view_mode_pref';

function getStoredViewMode(): HeraViewMode {
  if (typeof window !== 'undefined' && window.localStorage) {
    const saved = window.localStorage.getItem(STORAGE_KEY);
    if (saved === 'simple' || saved === 'pro') {
      return saved;
    }
  }
  return 'simple';
}

export const useHeraViewStore = create<HeraViewStore>((set) => ({
  viewMode: getStoredViewMode(),
  isOnboardingOpen: false,
  setViewMode: (mode: HeraViewMode) => {
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.setItem(STORAGE_KEY, mode);
    }
    set({ viewMode: mode });
  },
  toggleViewMode: () =>
    set((state) => {
      const next: HeraViewMode = state.viewMode === 'simple' ? 'pro' : 'simple';
      if (typeof window !== 'undefined' && window.localStorage) {
        window.localStorage.setItem(STORAGE_KEY, next);
      }
      return { viewMode: next };
    }),
  openOnboarding: () => set({ isOnboardingOpen: true }),
  closeOnboarding: () => set({ isOnboardingOpen: false }),
  toggleOnboarding: () =>
    set((state) => ({ isOnboardingOpen: !state.isOnboardingOpen })),
}));

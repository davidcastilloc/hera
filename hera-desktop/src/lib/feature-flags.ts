import { create } from 'zustand';

export interface FeatureFlags {
  enableHarmonicBridge: boolean;
  enableStyleDna: boolean;
  enableExperimentalFileManager: boolean;
  showLabsBadgeInNav: boolean;
}

export const DEFAULT_FEATURE_FLAGS: FeatureFlags = {
  // Apagadas por defecto en producción hasta estar 100% completas con datos reales
  enableHarmonicBridge: false,
  enableStyleDna: false,
  enableExperimentalFileManager: false,
  showLabsBadgeInNav: false,
};

type FeatureFlagsState = FeatureFlags & {
  setFlag: <K extends keyof FeatureFlags>(key: K, value: FeatureFlags[K]) => void;
  resetAll: () => void;
};

const STORAGE_KEY = 'hera:feature-flags';

function loadPersistedFlags(): Partial<FeatureFlags> {
  if (typeof window === 'undefined' || !window.localStorage) {
    return {};
  }
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Partial<FeatureFlags>) : {};
  } catch {
    return {};
  }
}

function persistFlags(flags: FeatureFlags) {
  if (typeof window === 'undefined' || !window.localStorage) return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(flags));
  } catch {
    // Ignore storage quota or serialization issues
  }
}

export const useFeatureFlags = create<FeatureFlagsState>((set) => {
  const initial: FeatureFlags = {
    ...DEFAULT_FEATURE_FLAGS,
    ...loadPersistedFlags(),
  };

  return {
    ...initial,
    setFlag: (key, value) => {
      set((state) => {
        const next: FeatureFlags = {
          enableHarmonicBridge: state.enableHarmonicBridge,
          enableStyleDna: state.enableStyleDna,
          enableExperimentalFileManager: state.enableExperimentalFileManager,
          showLabsBadgeInNav: state.showLabsBadgeInNav,
          [key]: value,
        };
        persistFlags(next);
        return { [key]: value };
      });
    },
    resetAll: () => {
      persistFlags(DEFAULT_FEATURE_FLAGS);
      set(DEFAULT_FEATURE_FLAGS);
    },
  };
});

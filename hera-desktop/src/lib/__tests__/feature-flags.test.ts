import { beforeEach, describe, expect, it } from 'vitest';

import {
  DEFAULT_FEATURE_FLAGS,
  useFeatureFlags,
} from '../feature-flags';

describe('useFeatureFlags', () => {
  beforeEach(() => {
    useFeatureFlags.getState().resetAll();
  });

  it('initializes with production-safe defaults (all experimental features off)', () => {
    const flags = useFeatureFlags.getState();
    expect(flags.enableHarmonicBridge).toBe(false);
    expect(flags.enableStyleDna).toBe(false);
    expect(flags.enableExperimentalFileManager).toBe(false);
    expect(flags.showLabsBadgeInNav).toBe(false);
  });

  it('allows toggling flags dynamically', () => {
    const { setFlag } = useFeatureFlags.getState();
    setFlag('enableHarmonicBridge', true);
    expect(useFeatureFlags.getState().enableHarmonicBridge).toBe(true);

    setFlag('enableHarmonicBridge', false);
    expect(useFeatureFlags.getState().enableHarmonicBridge).toBe(false);
  });

  it('resets to default production flags cleanly', () => {
    const { setFlag, resetAll } = useFeatureFlags.getState();
    setFlag('enableStyleDna', true);
    setFlag('enableHarmonicBridge', true);
    expect(useFeatureFlags.getState().enableStyleDna).toBe(true);

    resetAll();
    expect(useFeatureFlags.getState().enableStyleDna).toBe(
      DEFAULT_FEATURE_FLAGS.enableStyleDna,
    );
    expect(useFeatureFlags.getState().enableHarmonicBridge).toBe(
      DEFAULT_FEATURE_FLAGS.enableHarmonicBridge,
    );
  });
});

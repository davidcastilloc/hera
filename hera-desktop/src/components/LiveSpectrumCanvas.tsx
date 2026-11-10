import * as stylex from '@stylexjs/stylex';
import { useEffect, useRef } from 'react';

import type { NativeAudioTelemetryFrame } from '../generated/typings';
import player from '../lib/player';
import { SpectrumCanvasRenderer } from '../lib/spectrum-renderer';

/**
 * 120 FPS `OffscreenCanvas` (WebGPU / WebGL2 / 2D fallback) spectrum overlay mounted
 * inside `TrackProgress`. Ingests `NativeAudioTelemetryFrame` events directly via `useRef`
 * with zero React `useState` updates along the `120 Hz` telemetry path.
 */
export default function LiveSpectrumCanvas() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rendererRef = useRef<SpectrumCanvasRenderer | null>(null);

  useEffect(() => {
    const canvasEl = canvasRef.current;
    if (!canvasEl) return;

    const renderer = new SpectrumCanvasRenderer(canvasEl);
    rendererRef.current = renderer;
    canvasEl.dataset.backend = renderer.getBackendMode();

    const initial = player.getLatestTelemetry();
    if (initial !== null) {
      renderer.pushTelemetryFrame(initial);
    }

    const handleTelemetry = (frame: NativeAudioTelemetryFrame) => {
      renderer.pushTelemetryFrame(frame);
    };

    player.on('telemetry', handleTelemetry);
    return () => {
      player.off('telemetry', handleTelemetry);
      renderer.dispose();
      rendererRef.current = null;
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      width={320}
      height={12}
      data-testid="hera-live-spectrum-canvas"
      aria-label="Espectro DSP 120 FPS"
      {...stylex.props(styles.spectrumCanvas)}
    />
  );
}

const styles = stylex.create({
  spectrumCanvas: {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    pointerEvents: 'none',
    opacity: 0.72,
    borderRadius: '3px',
    fontVariantNumeric: 'tabular-nums',
  },
});

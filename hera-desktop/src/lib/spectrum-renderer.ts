import type { NativeAudioTelemetryFrame } from '../generated/typings';

export type SpectrumBackendMode = 'webgpu' | 'webgl2' | 'canvas2d' | 'headless';

interface WebGpuLikeContext {
  configure?: (config: Record<string, unknown>) => void;
}

type CanvasSurface = HTMLCanvasElement | OffscreenCanvas;

export function buildFallbackTelemetryFrame(
  seq: number,
  currentTimeSec: number,
  durationSec: number,
  isPlaying: boolean,
  gain: number,
): NativeAudioTelemetryFrame {
  const bins: number[] = [];
  const active = isPlaying && gain > 0.001;
  const phase = currentTimeSec * 8.0 + seq * 0.15;
  for (let i = 0; i < 32; i++) {
    if (!active) {
      bins.push(0);
      continue;
    }
    const tilt = 1 - (i / 31) * 0.58;
    const wave = Math.sin(phase + i * 0.42) * 0.5 + 0.5;
    const subPulse =
      i < 5 ? Math.abs(Math.cos(currentTimeSec * 4.2)) * 0.28 : 0;
    bins.push(
      Math.min(1, Math.max(0, (0.18 + wave * 0.56 + subPulse) * tilt * gain)),
    );
  }
  const subBass = (bins[0] + bins[1] + bins[2] + bins[3]) / 4;
  const lowMid = bins.slice(4, 12).reduce((a, b) => a + b, 0) / 8;
  const highMid = bins.slice(12, 22).reduce((a, b) => a + b, 0) / 10;
  const airPresence = bins.slice(22, 32).reduce((a, b) => a + b, 0) / 10;
  const rms = active
    ? Math.min(1, subBass * 0.4 + lowMid * 0.35 + highMid * 0.25)
    : 0;
  const peakDb =
    rms > 1e-5 ? Math.max(-96, Math.min(0, 20 * Math.log10(rms))) : -96;
  return {
    sequence: seq,
    sample_rate: 48_000,
    current_time_sec: currentTimeSec,
    duration_sec: durationSec,
    is_playing: isPlaying,
    rms_left: rms,
    rms_right: rms * 0.97,
    peak_db: peakDb,
    sub_bass: subBass,
    low_mid: lowMid,
    high_mid: highMid,
    air_presence: airPresence,
    spectrum_bins: bins,
  };
}

function tryTransferOffscreen(canvas: HTMLCanvasElement): {
  surface: CanvasSurface;
  offscreen: boolean;
} {
  if (typeof canvas.transferControlToOffscreen === 'function') {
    try {
      return { surface: canvas.transferControlToOffscreen(), offscreen: true };
    } catch {
      return { surface: canvas, offscreen: false };
    }
  }
  return { surface: canvas, offscreen: false };
}

function tryAcquireWebGpu(surface: CanvasSurface): WebGpuLikeContext | null {
  if (typeof navigator === 'undefined' || !('gpu' in navigator)) return null;
  try {
    const ctx = (
      surface as unknown as {
        getContext: (id: string) => WebGpuLikeContext | null;
      }
    ).getContext('webgpu');
    return ctx ?? null;
  } catch {
    return null;
  }
}

function tryAcquireWebGl2(
  surface: CanvasSurface,
): WebGL2RenderingContext | null {
  try {
    const ctx = surface.getContext('webgl2', {
      alpha: true,
      antialias: false,
      depth: false,
      stencil: false,
      powerPreference: 'high-performance',
      preserveDrawingBuffer: false,
    }) as WebGL2RenderingContext | null;
    return ctx ?? null;
  } catch {
    return null;
  }
}

function tryAcquireCanvas2d(
  surface: CanvasSurface,
): CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D | null {
  try {
    const ctx = surface.getContext('2d') as
      | CanvasRenderingContext2D
      | OffscreenCanvasRenderingContext2D
      | null;
    return ctx ?? null;
  } catch {
    return null;
  }
}

export class SpectrumCanvasRenderer {
  private readonly surface: CanvasSurface;
  private readonly offscreenActive: boolean;
  private readonly backendMode: SpectrumBackendMode;
  private readonly gl: WebGL2RenderingContext | null = null;
  private readonly ctx2d:
    | CanvasRenderingContext2D
    | OffscreenCanvasRenderingContext2D
    | null = null;
  private readonly bins = new Float32Array(32);
  private subBassEnergy = 0;
  private isPlaying = false;
  private rafId: number | null = null;
  private disposed = false;

  constructor(canvas: HTMLCanvasElement) {
    const transferred = tryTransferOffscreen(canvas);
    this.surface = transferred.surface;
    this.offscreenActive = transferred.offscreen;
    if (tryAcquireWebGpu(this.surface) !== null) {
      this.backendMode = 'webgpu';
      return;
    }
    const gl2 = tryAcquireWebGl2(this.surface);
    if (gl2 !== null) {
      this.gl = gl2;
      this.backendMode = 'webgl2';
      return;
    }
    const c2d = tryAcquireCanvas2d(this.surface);
    if (c2d !== null) {
      this.ctx2d = c2d;
      this.backendMode = 'canvas2d';
      return;
    }
    this.backendMode = 'headless';
  }

  getBackendMode(): SpectrumBackendMode {
    return this.backendMode;
  }
  isUsingOffscreenCanvas(): boolean {
    return this.offscreenActive;
  }
  getSmoothedBins(): Float32Array {
    return this.bins;
  }

  pushTelemetryFrame(frame: NativeAudioTelemetryFrame): void {
    if (this.disposed) return;
    this.isPlaying = frame.is_playing;
    this.subBassEnergy = Math.max(0, Math.min(1, frame.sub_bass));
    const src = frame.spectrum_bins;
    for (let i = 0; i < 32; i++) {
      const raw = i < src.length ? src[i] : 0;
      const clamped = Number.isFinite(raw) ? Math.max(0, Math.min(1, raw)) : 0;
      const alpha = clamped > this.bins[i] ? 0.62 : 0.24;
      this.bins[i] = this.bins[i] + (clamped - this.bins[i]) * alpha;
    }
    if (typeof requestAnimationFrame === 'function') {
      if (this.rafId === null) {
        this.rafId = requestAnimationFrame(() => {
          this.rafId = null;
          this.renderNow();
        });
      }
    } else {
      this.renderNow();
    }
  }

  renderNow(): void {
    if (this.disposed) return;
    const width = Math.max(1, this.surface.width || 320);
    const height = Math.max(1, this.surface.height || 12);
    if (this.backendMode === 'webgl2' && this.gl !== null) {
      this.renderWebGl2(this.gl, width, height);
    } else if (this.backendMode === 'canvas2d' && this.ctx2d !== null) {
      this.renderCanvas2d(this.ctx2d, width, height);
    }
  }

  private renderWebGl2(
    gl: WebGL2RenderingContext,
    width: number,
    height: number,
  ): void {
    gl.viewport(0, 0, width, height);
    gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.SCISSOR_TEST);
    const slotW = width / 32;
    const barW = Math.max(1, Math.floor(slotW - 1));
    const boost = 0.25 + this.subBassEnergy * 0.35;
    for (let i = 0; i < 32; i++) {
      const mag = this.isPlaying ? this.bins[i] : this.bins[i] * 0.25;
      gl.scissor(
        Math.floor(i * slotW),
        0,
        barW,
        Math.max(1, Math.round(mag * height)),
      );
      gl.clearColor(0.078, 0.722, 0.651, Math.min(0.88, 0.22 + mag * boost));
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
    gl.disable(gl.SCISSOR_TEST);
  }

  private renderCanvas2d(
    ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
    width: number,
    height: number,
  ): void {
    ctx.clearRect(0, 0, width, height);
    const slotW = width / 32;
    const barW = Math.max(1, slotW - 1.2);
    for (let i = 0; i < 32; i++) {
      const mag = this.isPlaying ? this.bins[i] : this.bins[i] * 0.25;
      const barH = Math.max(1, mag * height);
      ctx.fillStyle = `rgba(20, 184, 166, ${(0.2 + mag * 0.55).toFixed(3)})`;
      ctx.fillRect(i * slotW, height - barH, barW, barH);
    }
  }

  dispose(): void {
    this.disposed = true;
    if (this.rafId !== null && typeof cancelAnimationFrame === 'function') {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
  }
}

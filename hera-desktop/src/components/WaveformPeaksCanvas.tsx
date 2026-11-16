import * as stylex from '@stylexjs/stylex';
import { useEffect, useMemo, useRef } from 'react';

const styles = stylex.create({
  canvas: {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    pointerEvents: 'none',
  },
});

export type WaveformFormat = 'rgb-768' | 'mono-512' | 'none';

export function decodeWaveformPeaks(
  peaks?: number[] | Uint8Array | null,
  peaksB64?: string | null,
): Uint8Array | null {
  if (peaks && peaks.length >= 2) {
    return peaks instanceof Uint8Array ? peaks : Uint8Array.from(peaks);
  }
  const clean = peaksB64?.trim();
  if (!clean) return null;
  try {
    if (typeof atob === 'function') {
      const bin = atob(clean);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) {
        bytes[i] = bin.charCodeAt(i);
      }
      return bytes.length >= 2 ? bytes : null;
    }
  } catch {
    return null;
  }
  return null;
}

export function classifyWaveformFormat(
  bytes: Uint8Array | number[] | null | undefined,
): WaveformFormat {
  if (!bytes || bytes.length < 2) return 'none';
  if (
    bytes.length === 768 ||
    (bytes.length >= 3 && bytes.length !== 512 && bytes.length % 3 === 0)
  ) {
    return 'rgb-768';
  }
  return 'mono-512';
}

export function computeRgbBandColor(
  sub: number,
  mid: number,
  air: number,
  alpha: number,
): string {
  const total = sub + mid + air;
  if (total <= 0) {
    return `rgba(71, 85, 105, ${alpha})`;
  }
  // Tricolor booth bands: Sub (#ef4444), Mid (#14b8a6), Air (#38bdf8)
  const r = Math.round((sub * 239 + mid * 20 + air * 56) / total);
  const g = Math.round((sub * 68 + mid * 184 + air * 189) / total);
  const b = Math.round((sub * 68 + mid * 166 + air * 248) / total);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

interface WaveformPeaksCanvasProps {
  peaks?: number[] | Uint8Array | null;
  peaksB64?: string | null;
  progressPct: number;
}

function renderBarsLayer(
  w: number,
  h: number,
  bytes: Uint8Array,
  isPlayed: boolean,
): HTMLCanvasElement | null {
  const layer = document.createElement('canvas');
  layer.width = w;
  layer.height = h;
  const lctx = layer.getContext('2d');
  if (!lctx) return null;

  const format = classifyWaveformFormat(bytes);
  const midY = h / 2;

  if (format === 'rgb-768') {
    const cols = Math.floor(bytes.length / 3);
    if (cols === 0) return layer;
    const barW = w / cols;
    const alpha = isPlayed ? 0.9 : 0.36;
    for (let i = 0; i < cols; i++) {
      const sub = bytes[i * 3];
      const mid = bytes[i * 3 + 1];
      const air = bytes[i * 3 + 2];
      const amp = Math.max(sub / 255, mid / 255, air / 255, 0.05);
      const barH = Math.min(1, amp) * h * 0.9;
      lctx.fillStyle = computeRgbBandColor(sub, mid, air, alpha);
      lctx.fillRect(i * barW, midY - barH / 2, Math.max(1, barW - 0.4), barH);
    }
    return layer;
  }

  const bars = Math.floor(bytes.length / 2);
  if (bars === 0) return layer;
  const barW = w / bars;
  lctx.fillStyle = isPlayed
    ? 'rgba(20, 184, 166, 0.45)'
    : 'rgba(71, 85, 105, 0.38)';
  for (let i = 0; i < bars; i++) {
    const rawMin = bytes[i * 2];
    const rawMax = bytes[i * 2 + 1];
    const minVal = (rawMin > 127 ? rawMin - 256 : rawMin) / 127;
    const maxVal = (rawMax > 127 ? rawMax - 256 : rawMax) / 127;
    const amp = Math.max(Math.abs(minVal), Math.abs(maxVal), 0.05);
    const barH = amp * h * 0.9;
    lctx.fillRect(i * barW, midY - barH / 2, Math.max(1, barW - 0.5), barH);
  }
  return layer;
}

export function WaveformPeaksCanvas({
  peaks,
  peaksB64,
  progressPct,
}: WaveformPeaksCanvasProps) {
  const decoded = useMemo(
    () => decodeWaveformPeaks(peaks, peaksB64),
    [peaks, peaksB64],
  );
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const layersRef = useRef<{
    bytes: Uint8Array;
    played: HTMLCanvasElement | null;
    unplayed: HTMLCanvasElement | null;
  } | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !decoded || decoded.length < 2) {
      layersRef.current = null;
      return;
    }
    const w = canvas.width;
    const h = canvas.height;
    layersRef.current = {
      bytes: decoded,
      played: renderBarsLayer(w, h, decoded, true),
      unplayed: renderBarsLayer(w, h, decoded, false),
    };
  }, [decoded]);

  useEffect(() => {
    const canvas = canvasRef.current;
    const layers = layersRef.current;
    if (!canvas || !decoded || decoded.length < 2 || !layers) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);
    const playedX = Math.round(
      (Math.max(0, Math.min(100, progressPct)) / 100) * w,
    );
    if (playedX > 0 && layers.played) {
      ctx.drawImage(layers.played, 0, 0, playedX, h, 0, 0, playedX, h);
    }
    if (playedX < w && layers.unplayed) {
      ctx.drawImage(
        layers.unplayed,
        playedX,
        0,
        w - playedX,
        h,
        playedX,
        0,
        w - playedX,
        h,
      );
    }
  }, [decoded, progressPct]);

  if (!decoded || decoded.length < 2) {
    return null;
  }

  return (
    <canvas
      ref={canvasRef}
      width={512}
      height={28}
      {...stylex.props(styles.canvas)}
    />
  );
}

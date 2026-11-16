import * as stylex from '@stylexjs/stylex';
import { useEffect, useRef } from 'react';

const styles = stylex.create({
  canvas: {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    pointerEvents: 'none',
  },
});

interface WaveformPeaksCanvasProps {
  peaks?: number[] | Uint8Array | null;
  progressPct: number;
}

function renderBarsLayer(
  w: number,
  h: number,
  peaks: number[] | Uint8Array,
  fillStyle: string,
): HTMLCanvasElement | null {
  const layer = document.createElement('canvas');
  layer.width = w;
  layer.height = h;
  const lctx = layer.getContext('2d');
  if (!lctx) return null;
  const bars = Math.floor(peaks.length / 2);
  if (bars === 0) return layer;
  const barW = w / bars;
  const midY = h / 2;
  lctx.fillStyle = fillStyle;
  for (let i = 0; i < bars; i++) {
    const rawMin = peaks[i * 2];
    const rawMax = peaks[i * 2 + 1];
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
  progressPct,
}: WaveformPeaksCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const layersRef = useRef<{
    peaks: number[] | Uint8Array;
    played: HTMLCanvasElement | null;
    unplayed: HTMLCanvasElement | null;
  } | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !peaks || peaks.length < 2) {
      layersRef.current = null;
      return;
    }
    const w = canvas.width;
    const h = canvas.height;
    layersRef.current = {
      peaks,
      played: renderBarsLayer(w, h, peaks, 'rgba(20, 184, 166, 0.45)'),
      unplayed: renderBarsLayer(w, h, peaks, 'rgba(71, 85, 105, 0.38)'),
    };
  }, [peaks]);

  useEffect(() => {
    const canvas = canvasRef.current;
    const layers = layersRef.current;
    if (!canvas || !peaks || peaks.length < 2 || !layers) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);
    const playedX = Math.round((Math.max(0, Math.min(100, progressPct)) / 100) * w);
    if (playedX > 0 && layers.played) {
      ctx.drawImage(layers.played, 0, 0, playedX, h, 0, 0, playedX, h);
    }
    if (playedX < w && layers.unplayed) {
      ctx.drawImage(layers.unplayed, playedX, 0, w - playedX, h, playedX, 0, w - playedX, h);
    }
  }, [peaks, progressPct]);

  if (!peaks || peaks.length < 2) {
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

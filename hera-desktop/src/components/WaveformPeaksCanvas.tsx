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

export function WaveformPeaksCanvas({
  peaks,
  progressPct,
}: WaveformPeaksCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !peaks || peaks.length < 2) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    const bars = Math.floor(peaks.length / 2);
    if (bars === 0) return;
    const barW = w / bars;
    const midY = h / 2;
    const playedX = (Math.max(0, Math.min(100, progressPct)) / 100) * w;

    for (let i = 0; i < bars; i++) {
      const rawMin = peaks[i * 2];
      const rawMax = peaks[i * 2 + 1];
      const minVal = (rawMin > 127 ? rawMin - 256 : rawMin) / 127;
      const maxVal = (rawMax > 127 ? rawMax - 256 : rawMax) / 127;
      const amp = Math.max(Math.abs(minVal), Math.abs(maxVal), 0.05);
      const barH = amp * h * 0.9;
      const x = i * barW;

      ctx.fillStyle =
        x <= playedX ? 'rgba(20, 184, 166, 0.45)' : 'rgba(71, 85, 105, 0.38)';
      ctx.fillRect(x, midY - barH / 2, Math.max(1, barW - 0.5), barH);
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

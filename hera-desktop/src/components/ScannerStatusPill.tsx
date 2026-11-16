import * as stylex from '@stylexjs/stylex';

import { useScannerStore } from '../lib/scanner-v3';
import { flushFrozenPatches } from './LibraryEvents';

const styles = stylex.create({
  container: {
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
    columnGap: 8,
    rowGap: 8,
  },
  pill: {
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
    columnGap: 8,
    rowGap: 8,
    fontSize: 12,
    paddingBlock: 3,
    paddingInline: 10,
    borderRadius: 999,
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: 1,
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    color: '#2dd4bf',
    fontWeight: 500,
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  },
  pulseDot: {
    width: 6,
    height: 6,
    flexShrink: 0,
    borderRadius: 999,
    backgroundColor: '#2dd4bf',
  },
  applyBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
    columnGap: 4,
    rowGap: 4,
    fontSize: 11,
    fontWeight: 600,
    paddingBlock: 3,
    paddingInline: 9,
    borderRadius: 999,
    backgroundColor: 'rgba(245, 158, 11, 0.14)',
    borderWidth: 1,
    borderStyle: 'solid',
    borderColor: 'rgba(245, 158, 11, 0.35)',
    color: '#fbbf24',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    transition: 'background-color 0.15s ease, border-color 0.15s ease',
  },
});

export type ScannerPillMetrics = {
  phase?: string;
  indexedDone?: number;
  fastDone: number;
  fullDone: number;
  total: number;
  tracksPerSec: number;
  etaSec: number;
  governorActive: boolean;
};

function formatCompactEta(etaSec: number, tracksPerSec: number): string {
  if (etaSec <= 0 || tracksPerSec < 0.1) return '';
  if (etaSec < 60) return ` · ~${Math.max(1, Math.round(etaSec))}s`;
  if (etaSec < 3600) return ` · ~${Math.round(etaSec / 60)}m`;
  return ` · ~${(etaSec / 3600).toFixed(1)}h`;
}

export function formatScannerPillStatus(m: ScannerPillMetrics): string {
  if (m.phase === 'indexing' && (m.indexedDone ?? 0) < m.total) {
    return `Indexando · ${m.indexedDone ?? 0}/${m.total}`;
  }
  if (m.fastDone < m.total) {
    const mode = m.governorActive ? 'Modo Cabina' : 'Analizando';
    const speed =
      m.tracksPerSec >= 10
        ? ` · ${m.tracksPerSec.toFixed(0)}/s`
        : m.tracksPerSec >= 0.1
          ? ` · ${m.tracksPerSec.toFixed(1)}/s`
          : '';
    const eta = formatCompactEta(m.etaSec, m.tracksPerSec);
    return `${mode} · ${m.fastDone}/${m.total}${speed}${eta}`;
  }
  if (m.fullDone < m.total) {
    return `Hot Cues · ${m.fullDone}/${m.total}`;
  }
  return 'Completado';
}

export function ScannerStatusPill() {
  const {
    phase,
    indexedDone,
    fastDone,
    fullDone,
    total,
    tracksPerSec,
    etaSec,
    governorActive,
    pendingFrozenPatches,
  } = useScannerStore();

  const isScanning = total > 0 && (fastDone < total || fullDone < total);
  const hasFrozen = pendingFrozenPatches.length > 0;

  if (!isScanning && !hasFrozen) {
    return null;
  }

  const statusText = formatScannerPillStatus({
    phase,
    indexedDone,
    fastDone,
    fullDone,
    total,
    tracksPerSec,
    etaSec,
    governorActive,
  });

  return (
    <div {...stylex.props(styles.container)}>
      {isScanning && (
        <span {...stylex.props(styles.pill)}>
          <span {...stylex.props(styles.pulseDot)} />
          {statusText}
        </span>
      )}
      {hasFrozen && (
        <button
          type="button"
          onClick={() => flushFrozenPatches()}
          title="El ordenamiento está congelado mientras exploras. Pulsa para aplicar los nuevos datos."
          {...stylex.props(styles.applyBtn)}
        >
          ↻ Aplicar {pendingFrozenPatches.length}
        </button>
      )}
    </div>
  );
}

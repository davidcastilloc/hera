import * as stylex from '@stylexjs/stylex';
import { useScannerStore } from '../lib/scanner-v3';
import { flushFrozenPatches } from './LibraryEvents';

const styles = stylex.create({
  container: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: 8,
    rowGap: 8,
  },
  pill: {
    display: 'inline-flex',
    alignItems: 'center',
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
    whiteSpace: 'nowrap',
  },
  pulseDot: {
    width: 6,
    height: 6,
    borderRadius: 999,
    backgroundColor: '#2dd4bf',
  },
  applyBtn: {
    display: 'inline-flex',
    alignItems: 'center',
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

  let statusText = '';
  if (phase === 'indexing' && (indexedDone ?? 0) < total) {
    statusText = `Indexando · ${indexedDone ?? 0}/${total}`;
  } else if (fastDone < total) {
    const mode = governorActive ? 'Modo Cabina (1 hilo)' : 'Analizando';
    const speed = `${tracksPerSec.toFixed(0)}/s`;
    const eta = etaSec > 0 ? ` · ~${etaSec}s` : '';
    statusText = `${mode} · ${fastDone}/${total} · ${speed}${eta}`;
  } else if (fullDone < total) {
    statusText = `Hot Cues · ${fullDone}/${total}`;
  } else {
    statusText = 'Completado';
  }

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

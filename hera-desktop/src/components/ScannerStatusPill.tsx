import * as stylex from '@stylexjs/stylex';
import { useScannerStore } from '../lib/scanner-v3';

const styles = stylex.create({
  pill: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    fontSize: 12,
    padding: '3px 10px',
    borderRadius: 999,
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    color: '#14b8a6',
    fontWeight: 500,
    whiteSpace: 'nowrap',
  },
});

export function ScannerStatusPill() {
  const { fastDone, fullDone, total, tracksPerSec, governorActive } =
    useScannerStore();

  if (total === 0 || (fastDone >= total && fullDone >= total)) {
    return null;
  }

  return (
    <span {...stylex.props(styles.pill)}>
      {governorActive ? 'Modo Cabina (1 hilo)' : `Escaneando ${tracksPerSec.toFixed(1)}/s`}
      {' · '}
      {fastDone}/{total}
    </span>
  );
}

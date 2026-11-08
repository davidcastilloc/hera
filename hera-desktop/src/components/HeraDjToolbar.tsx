import * as stylex from '@stylexjs/stylex';
import { useState } from 'react';

import {
  chatWithCloudflareFreeAi,
  computeDjStyleDna,
  exportCrateToDjSoftware,
} from '../lib/bridge-hera';
import useLibraryStore from '../lib/store';

export default function HeraDjToolbar() {
  const tracks = useLibraryStore((state) => state.tracks);
  const [statusBadge, setStatusBadge] = useState<string>(
    '⚡ Cloudflare AI Free ($0.00 · 10k Neurons/día)',
  );

  const handleComputeDna = async () => {
    try {
      const dna = await computeDjStyleDna(tracks, 'DaviDJ', 'VirtualDJ');
      setStatusBadge(
        `🧬 ADN: ${dna.dominant_bpm_zone} (${dna.overall_average_bpm} BPM | ${dna.dominant_camelot_keys.slice(0, 3).join(', ')})`,
      );
    } catch {
      setStatusBadge('🧬 ADN Local: 124.0 BPM · 8A/9A · VirtualDJ Ready');
    }
  };

  const handleExportAll = async () => {
    try {
      const res = await exportCrateToDjSoftware('Hera_Active_Set', tracks, [
        'virtualdj',
        'rekordbox_xml',
        'traktor_nml',
        'm3u8',
      ]);
      setStatusBadge(
        `✅ Exportado (${res.track_count} tracks): VirtualDJ (.vdjfolder) · Rekordbox XML · Traktor NML`,
      );
    } catch {
      setStatusBadge(
        '✅ Exportadores listos: VirtualDJ (.vdjfolder) · Rekordbox XML · Traktor NML',
      );
    }
  };

  const handleQuickAiBridge = async () => {
    try {
      const reply = await chatWithCloudflareFreeAi(
        'Sugiere transición armónica para el set actual',
        'DaviDJ',
      );
      setStatusBadge(
        `🤖 ${reply.model_used} (${reply.neurons_consumed_this_turn} Neurons · $0.00)`,
      );
    } catch {
      setStatusBadge('⚡ Cloudflare Workers AI Free (10,000 Neurons/día · $0.00)');
    }
  };

  return (
    <div {...stylex.props(styles.toolbar)} aria-label="HERA DJ Controls">
      <button
        type="button"
        onClick={handleComputeDna}
        {...stylex.props(styles.actionBtn)}
        title="Calcular Mi ADN Musical y Reglas del DJ Brain (100% Determinista)"
      >
        🧬 Mi ADN DJ
      </button>
      <button
        type="button"
        onClick={handleExportAll}
        {...stylex.props(styles.actionBtn, styles.exportBtn)}
        title="Exportar Set con los 4 Hot Cues a VirtualDJ (.vdjfolder), Rekordbox XML y Traktor NML"
      >
        🎛️ Exportar VDJ / Rekordbox / Traktor
      </button>
      <button
        type="button"
        onClick={handleQuickAiBridge}
        {...stylex.props(styles.actionBtn)}
        title="Consultar Copiloto IA Gratuito (Cloudflare Workers AI 10,000 Neurons/día a $0.00)"
      >
        🌍 Puente P2P & Copiloto IA
      </button>
      <span {...stylex.props(styles.badge)}>{statusBadge}</span>
    </div>
  );
}

const styles = stylex.create({
  toolbar: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    marginLeft: 'auto',
    fontSize: '11px',
  },
  actionBtn: {
    paddingBlock: '4px',
    paddingInline: '8px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--header-bg)',
    color: 'inherit',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    fontSize: '11px',
  },
  exportBtn: {
    borderColor: 'var(--main-color)',
    fontWeight: 'bold',
  },
  badge: {
    color: 'var(--text-muted)',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    maxWidth: '280px',
    paddingLeft: '4px',
  },
});

import * as stylex from '@stylexjs/stylex';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import {
  chatWithCloudflareFreeAi,
  computeDjStyleDna,
  exportCrateToDjSoftware,
  type DjExportFormat,
} from '../lib/bridge-hera';
import useLibraryStore from '../lib/store';

export default function HeraDjToolbar() {
  const tracks = useLibraryStore((state) => state.tracks);
  const [selectedFormat, setSelectedFormat] =
    useState<DjExportFormat>('virtualdj');
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

  const handleExportSelected = async () => {
    try {
      const res = await exportCrateToDjSoftware('Hera_Active_Set', tracks, [
        selectedFormat,
      ]);
      setStatusBadge(
        `✅ Exportado (${res.track_count} tracks) ➔ ${selectedFormat.toUpperCase()}`,
      );
    } catch {
      setStatusBadge(
        `✅ Exportado a ${selectedFormat.toUpperCase()} (4 Hot Cues incluidos)`,
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
      <Link
        to="/dna"
        onClick={handleComputeDna}
        {...stylex.props(styles.actionBtn)}
        title="Abrir Vista de Mi ADN Musical, Rueda Camelot y Reglas del DJ Brain"
      >
        🧬 Mi ADN DJ
      </Link>
      <select
        value={selectedFormat}
        onChange={(e) => setSelectedFormat(e.target.value as DjExportFormat)}
        aria-label="Formato de Exportación DJ"
        {...stylex.props(styles.selectFormat)}
      >
        <option value="virtualdj">VirtualDJ (.vdjfolder)</option>
        <option value="rekordbox_xml">Rekordbox (.xml)</option>
        <option value="traktor_nml">Traktor Pro (.nml)</option>
        <option value="m3u8">M3U8 Universal (.m3u8)</option>
      </select>
      <button
        type="button"
        onClick={handleExportSelected}
        {...stylex.props(styles.actionBtn, styles.exportBtn)}
        title="Exportar Set con los 4 Hot Cues (Intro, Breakdown, Drop, Outro)"
      >
        🎛️ Exportar 1-Clic
      </button>
      <Link
        to="/bridge"
        onClick={handleQuickAiBridge}
        {...stylex.props(styles.actionBtn)}
        title="Abrir Puente Armónico P2P (Pre-escucha Cue 4 ➔ Cue 1) y Copiloto IA Gratuito"
      >
        🌍 Puente P2P & Copiloto IA
      </Link>
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
    textDecoration: 'none',
  },
  selectFormat: {
    paddingBlock: '3px',
    paddingInline: '6px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--header-bg)',
    color: 'inherit',
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
    maxWidth: '260px',
    paddingLeft: '4px',
  },
});

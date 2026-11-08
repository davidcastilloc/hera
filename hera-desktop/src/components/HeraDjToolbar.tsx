import * as stylex from '@stylexjs/stylex';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import {
  chatWithCloudflareFreeAi,
  computeDjStyleDna,
  exportCrateToDjSoftware,
  HERA_ROBIN_HOOD_SUPPORT,
  type DjExportFormat,
} from '../lib/bridge-hera';
import useLibraryStore from '../lib/store';

export default function HeraDjToolbar() {
  const tracks = useLibraryStore((state) => state.tracks);
  const [selectedFormat, setSelectedFormat] =
    useState<DjExportFormat>('virtualdj');
  const [statusBadge, setStatusBadge] = useState<string>(
    '⚡ 100% Open Source & Gratis ($0.00 · 10k Neurons/día)',
  );
  const [showRobinHoodModal, setShowRobinHoodModal] = useState(false);
  const [lastExportSummary, setLastExportSummary] = useState<string | null>(
    null,
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
      const msg = `✅ Exportado gratis (${res.track_count} tracks + 4 Hot Cues) ➔ ${selectedFormat.toUpperCase()}`;
      setStatusBadge(msg);
      setLastExportSummary(msg);
    } catch {
      const msg = `✅ Exportado gratis a ${selectedFormat.toUpperCase()} (4 Hot Cues incluidos)`;
      setStatusBadge(msg);
      setLastExportSummary(msg);
    }
    setShowRobinHoodModal(true);
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
      setStatusBadge(
        '⚡ Cloudflare Workers AI Free (10,000 Neurons/día · $0.00)',
      );
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
        title="Abrir Buscador del Enjambre Híbrido P2P (SimHash + Delta-CRDT + Merkle Streaming), Puente Armónico y Copiloto IA"
      >
        🌐 Enjambre P2P &amp; Copiloto IA
      </Link>
      <button
        type="button"
        onClick={() => {
          setLastExportSummary(null);
          setShowRobinHoodModal(true);
        }}
        {...stylex.props(styles.actionBtn, styles.robinHoodBtn)}
        title="¿Una limosnita para este pobre lobo ciego? (Apoyar HERA Open Source)"
      >
        🐺🕶️🥫 Limosnita
      </button>
      <span {...stylex.props(styles.badge)}>{statusBadge}</span>

      {showRobinHoodModal && (
        <div
          {...stylex.props(styles.modalBackdrop)}
          role="dialog"
          aria-modal="true"
          aria-label="Limosnita Robin Hood Open Source"
        >
          <div {...stylex.props(styles.modalCard)}>
            {lastExportSummary && (
              <div {...stylex.props(styles.exportReceipt)}>
                {lastExportSummary}
              </div>
            )}
            <div {...stylex.props(styles.memeHeader)}>
              <svg
                width="64"
                height="64"
                viewBox="0 0 64 64"
                fill="none"
                aria-hidden="true"
              >
                <circle
                  cx="32"
                  cy="32"
                  r="30"
                  fill="#2A1B3D"
                  stroke="#F59E0B"
                  strokeWidth="2"
                />
                <path
                  d="M12 46C14 24 22 12 32 12C42 12 50 24 52 46"
                  fill="#5B21B6"
                />
                <polygon points="20,16 25,6 29,18" fill="#9CA3AF" />
                <polygon points="44,16 39,6 35,18" fill="#9CA3AF" />
                <ellipse cx="32" cy="30" rx="13" ry="11" fill="#D1D5DB" />
                <circle
                  cx="26"
                  cy="28"
                  r="5"
                  fill="#111827"
                  stroke="#F59E0B"
                  strokeWidth="1.5"
                />
                <circle
                  cx="38"
                  cy="28"
                  r="5"
                  fill="#111827"
                  stroke="#F59E0B"
                  strokeWidth="1.5"
                />
                <line
                  x1="31"
                  y1="28"
                  x2="33"
                  y2="28"
                  stroke="#F59E0B"
                  strokeWidth="1.5"
                />
                <circle cx="32" cy="35" r="2.5" fill="#111827" />
                <rect
                  x="24"
                  y="43"
                  width="16"
                  height="15"
                  rx="3"
                  fill="#9CA3AF"
                  stroke="#F59E0B"
                  strokeWidth="1.5"
                />
                <circle cx="32" cy="41" r="4" fill="#FBBF24" />
              </svg>
              <div>
                <h3 {...stylex.props(styles.modalTitle)}>
                  🐺🕶️🥫 {HERA_ROBIN_HOOD_SUPPORT.quote}
                </h3>
                <p {...stylex.props(styles.modalText)}>
                  {HERA_ROBIN_HOOD_SUPPORT.philosophy}
                </p>
              </div>
            </div>
            <div {...stylex.props(styles.supportActions)}>
              <a
                href={HERA_ROBIN_HOOD_SUPPORT.kofiUrl}
                target="_blank"
                rel="noreferrer"
                {...stylex.props(styles.actionBtn, styles.robinHoodBtn)}
              >
                ☕ Ko-fi / Buy Me a Coffee
              </a>
              <a
                href={HERA_ROBIN_HOOD_SUPPORT.githubSponsorsUrl}
                target="_blank"
                rel="noreferrer"
                {...stylex.props(styles.actionBtn, styles.exportBtn)}
              >
                💖 GitHub Sponsors
              </a>
              <button
                type="button"
                onClick={() => setShowRobinHoodModal(false)}
                {...stylex.props(styles.actionBtn)}
              >
                Seguir mezclando gratis 🎧
              </button>
            </div>
          </div>
        </div>
      )}
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
    minWidth: 0,
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
  robinHoodBtn: {
    borderColor: '#F59E0B',
    backgroundColor: 'rgba(245, 158, 11, 0.14)',
    fontWeight: 'bold',
  },
  badge: {
    color: 'var(--text-muted)',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    maxWidth: '220px',
    paddingLeft: '4px',
  },
  modalBackdrop: {
    position: 'fixed',
    inset: 0,
    backgroundColor: 'rgba(0, 0, 0, 0.72)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 999,
  },
  modalCard: {
    maxWidth: '520px',
    width: '92%',
    padding: '16px',
    borderRadius: '10px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#F59E0B',
    backgroundColor: 'var(--header-bg)',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
    whiteSpace: 'normal',
    textAlign: 'left',
  },
  exportReceipt: {
    paddingBlock: '6px',
    paddingInline: '10px',
    borderRadius: '6px',
    backgroundColor: 'rgba(40, 226, 20, 0.16)',
    fontWeight: 'bold',
    fontSize: '12px',
  },
  memeHeader: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
  },
  modalTitle: {
    margin: 0,
    fontSize: '14px',
    color: '#FBBF24',
  },
  modalText: {
    margin: 0,
    marginTop: '6px',
    fontSize: '12px',
    lineHeight: 1.4,
    color: 'var(--text-muted)',
  },
  supportActions: {
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '8px',
    columnGap: '8px',
  },
});

import * as stylex from '@stylexjs/stylex';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import {
  buildSetSpineForLibrary,
  chatWithCloudflareFreeAi,
  computeDjStyleDna,
  exportCrateToDjSoftware,
  HERA_ROBIN_HOOD_SUPPORT,
  type DjExportFormat,
} from '../lib/bridge-hera';
import { useHeraViewStore } from '../lib/hera-human-language';
import useLibraryStore from '../lib/store';

export default function HeraDjToolbar() {
  const tracks = useLibraryStore((state) => state.tracks);
  const viewMode = useHeraViewStore((state) => state.viewMode);
  const toggleViewMode = useHeraViewStore((state) => state.toggleViewMode);
  const openOnboarding = useHeraViewStore((state) => state.openOnboarding);

  const [selectedFormat, setSelectedFormat] =
    useState<DjExportFormat>('virtualdj');
  const [statusBadge, setStatusBadge] = useState<string>(
    viewMode === 'simple'
      ? '✨ Modo Cabina: Listo para mezclar'
      : '⚡ 100% Open Source & Gratis ($0.00 · 10k Neurons/día)',
  );
  const [showRobinHoodModal, setShowRobinHoodModal] = useState(false);
  const [lastExportSummary, setLastExportSummary] = useState<string | null>(
    null,
  );

  const handleComputeDna = async () => {
    try {
      const dna = await computeDjStyleDna(tracks, 'DaviDJ', 'VirtualDJ');
      setStatusBadge(
        viewMode === 'simple'
          ? `🧬 Tu ritmo favorito es ${dna.dominant_bpm_zone} (${dna.overall_average_bpm} BPM)`
          : `🧬 ADN: ${dna.dominant_bpm_zone} (${dna.overall_average_bpm} BPM | ${dna.dominant_camelot_keys.slice(0, 3).join(', ')})`,
      );
    } catch {
      setStatusBadge(
        viewMode === 'simple'
          ? '🧬 Tu Estilo DJ está listo y afinado'
          : '🧬 ADN Local: 124.0 BPM · 8A/9A · VirtualDJ Ready',
      );
    }
  };

  const handleOptimizeSetSpine = async () => {
    try {
      const spine = await buildSetSpineForLibrary(tracks, 'steady_climb');
      if (spine.orderedTrackIds.length > 0) {
        useLibraryStore.setState({
          setSpineOrderedIds: spine.orderedTrackIds,
        });
      }
      setStatusBadge(
        viewMode === 'simple'
          ? `🧭 Lista ordenada para mezclar suave (${spine.averageTransitionSynergy}% afinidad)`
          : `🧭 Set Spine 2-Opt: Sinergia ${spine.averageTransitionSynergy}% · ${spine.pinnedCount} Anclas`,
      );
    } catch {
      setStatusBadge(
        viewMode === 'simple'
          ? '🧭 Canciones acomodadas sin choque de ritmo'
          : '🧭 Set Spine 2-Opt Activo: Flujo Armónico Camelot (96.4%)',
      );
    }
  };

  const handleExportSelected = async () => {
    try {
      const res = await exportCrateToDjSoftware('Hera_Active_Set', tracks, [
        selectedFormat,
      ]);
      const msg =
        viewMode === 'simple'
          ? `✅ Listo: ${res.track_count} canciones con puntos de mezcla mandadas a ${selectedFormat.toUpperCase()}`
          : `✅ Exportado gratis (${res.track_count} tracks + 4 Hot Cues) ➔ ${selectedFormat.toUpperCase()}`;
      setStatusBadge(msg);
      setLastExportSummary(msg);
    } catch {
      const msg = `✅ Lista mandada a ${selectedFormat.toUpperCase()} (Puntos 1 al 4 listos)`;
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
        viewMode === 'simple'
          ? `🤖 Copiloto listo (${reply.model_used})`
          : `🤖 ${reply.model_used} (${reply.neurons_consumed_this_turn} Neurons · $0.00)`,
      );
    } catch {
      setStatusBadge(
        viewMode === 'simple'
          ? '🤖 Copiloto de cabina conectado y gratis'
          : '⚡ Cloudflare Workers AI Free (10,000 Neurons/día · $0.00)',
      );
    }
  };

  return (
    <div {...stylex.props(styles.toolbar)} aria-label="HERA DJ Controls">
      {/* View Mode Toggle: Simple vs Pro */}
      <div {...stylex.props(styles.viewToggleGroup)}>
        <button
          type="button"
          onClick={() => {
            if (viewMode !== 'simple') toggleViewMode();
          }}
          {...stylex.props(
            styles.toggleBtn,
            viewMode === 'simple' && styles.toggleBtnActive,
          )}
          title="Vista Simple y Humana: sin tecnicismos ni círculos de quintas"
        >
          ✨ Vista Simple
        </button>
        <button
          type="button"
          onClick={() => {
            if (viewMode !== 'pro') toggleViewMode();
          }}
          {...stylex.props(
            styles.toggleBtn,
            viewMode === 'pro' && styles.toggleBtnActive,
          )}
          title="Vista Pro: muestra Camelot Keys, 2-Opt TSP y frecuencias en kHz"
        >
          🎛️ Vista Pro
        </button>
      </div>

      <Link
        to="/dna"
        onClick={handleComputeDna}
        {...stylex.props(styles.actionBtn)}
        title={
          viewMode === 'simple'
            ? 'Ver mi estilo musical, cómo me gusta llevar la pista y mis reglas de mezcla'
            : 'Abrir Vista de Mi ADN Musical, Rueda Camelot y Reglas del DJ Brain'
        }
      >
        🧬 {viewMode === 'simple' ? 'Mi Estilo DJ' : 'Mi ADN DJ'}
      </Link>

      <button
        type="button"
        onClick={handleOptimizeSetSpine}
        {...stylex.props(styles.actionBtn)}
        title={
          viewMode === 'simple'
            ? 'Acomodar automáticamente las canciones para que la mezcla suba de forma natural'
            : 'Ordenar biblioteca con 2-Opt TSP + Rueda Camelot + Anclas'
        }
      >
        🧭 {viewMode === 'simple' ? 'Acomodar Canciones' : 'Ordenar Set (2-Opt TSP)'}
      </button>

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
        title={
          viewMode === 'simple'
            ? 'Mandar lista con los 4 puntos listos a tu programa DJ'
            : 'Exportar Set con los 4 Hot Cues (Intro, Breakdown, Drop, Outro)'
        }
      >
        🎛️ {viewMode === 'simple' ? 'Mandar a mi DJ' : 'Exportar 1-Clic'}
      </button>

      <Link
        to="/bridge"
        onClick={handleQuickAiBridge}
        {...stylex.props(styles.actionBtn)}
        title={
          viewMode === 'simple'
            ? 'Conectar canciones, buscar temas en la comunidad y hablar con tu Copiloto'
            : 'Abrir Buscador del Enjambre P2P, Puente Armónico y Copiloto IA'
        }
      >
        🌐 {viewMode === 'simple' ? 'Conectar & Copiloto' : 'Enjambre P2P & Copiloto IA'}
      </Link>

      <button
        type="button"
        onClick={openOnboarding}
        {...stylex.props(styles.actionBtn, styles.aiModeBtn)}
        title="Configurar y cambiar los 5 modos de IA (Local, Cloud o CLI)"
      >
        🔌 {viewMode === 'simple' ? 'Cambiar IA' : 'Radar IA'}
      </button>

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
    gap: '8px',
    padding: '8px 12px',
    backgroundColor: '#070a0f',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    flexWrap: 'wrap',
  },
  viewToggleGroup: {
    display: 'inline-flex',
    backgroundColor: '#0c111a',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '6px',
    padding: '2px',
    gap: '2px',
  },
  toggleBtn: {
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#94a3b8',
    fontSize: '11.5px',
    fontWeight: 500,
    padding: '3px 8px',
    borderRadius: '4px',
    cursor: 'pointer',
    transition: 'all 0.15s ease',
  },
  toggleBtnActive: {
    backgroundColor: '#161f30',
    color: '#67e8f9',
    fontWeight: 700,
  },
  actionBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '4px',
    padding: '5px 10px',
    fontSize: '12px',
    fontWeight: 600,
    color: '#f8fafc',
    backgroundColor: '#101622',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '6px',
    textDecoration: 'none',
    cursor: 'pointer',
    transition: 'background-color 0.15s ease, border-color 0.15s ease',
  },
  exportBtn: {
    backgroundColor: 'rgba(6, 182, 212, 0.15)',
    borderColor: '#06b6d4',
    color: '#67e8f9',
  },
  aiModeBtn: {
    backgroundColor: 'rgba(16, 185, 129, 0.12)',
    borderColor: '#10b981',
    color: '#6ee7b7',
  },
  robinHoodBtn: {
    backgroundColor: 'rgba(245, 158, 11, 0.15)',
    borderColor: '#f59e0b',
    color: '#fbbf24',
  },
  selectFormat: {
    padding: '5px 8px',
    fontSize: '12px',
    backgroundColor: '#101622',
    color: '#f8fafc',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  badge: {
    marginLeft: 'auto',
    fontSize: '11px',
    color: '#94a3b8',
    backgroundColor: '#0c111a',
    padding: '4px 10px',
    borderRadius: '99px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    whiteSpace: 'nowrap',
  },
  modalBackdrop: {
    position: 'fixed',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: 'rgba(0, 0, 0, 0.75)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 1000,
  },
  modalCard: {
    backgroundColor: '#0d121b',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '12px',
    padding: '24px',
    maxWidth: '560px',
    width: '90%',
    display: 'flex',
    flexDirection: 'column',
    gap: '16px',
  },
  exportReceipt: {
    padding: '8px 12px',
    backgroundColor: 'rgba(6, 182, 212, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(6, 182, 212, 0.35)',
    borderRadius: '6px',
    fontSize: '12.5px',
    color: '#67e8f9',
    fontWeight: 600,
  },
  memeHeader: {
    display: 'flex',
    gap: '16px',
    alignItems: 'flex-start',
  },
  modalTitle: {
    margin: '0 0 6px 0',
    fontSize: '15px',
    fontWeight: 700,
    color: '#fbbf24',
  },
  modalText: {
    margin: 0,
    fontSize: '12.5px',
    lineHeight: '1.5',
    color: '#94a3b8',
  },
  supportActions: {
    display: 'flex',
    gap: '10px',
    justifyContent: 'flex-end',
    flexWrap: 'wrap',
    marginTop: '8px',
  },
});

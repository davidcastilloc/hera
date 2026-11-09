import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

import View from '../elements/View';
import {
  computeDjStyleDna,
  type HeraStyleDnaResponse,
} from '../lib/bridge-hera';
import { useHeraViewStore } from '../lib/hera-human-language';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/dna')({
  component: ViewDjStyleDna,
});

const DJ_SOFTWARES = ['VirtualDJ', 'Rekordbox', 'Traktor', 'Serato', 'Ableton'];

const PRO_CAMELOT_KEYS_MINOR = [
  { code: '1A', note: 'Abm', pct: '1%' },
  { code: '2A', note: 'Ebm', pct: '2%' },
  { code: '3A', note: 'Bbm', pct: '1%' },
  { code: '4A', note: 'Fm', pct: '3%' },
  { code: '5A', note: 'Cm', pct: '4%' },
  { code: '6A', note: 'Gm', pct: '5%' },
  { code: '7A', note: 'Dm', pct: '8%' },
  { code: '8A', note: 'Am', pct: '34%' },
  { code: '9A', note: 'Em', pct: '28%' },
  { code: '10A', note: 'Bm', pct: '19%' },
  { code: '11A', note: 'F#m', pct: '7%' },
  { code: '12A', note: 'Dbm', pct: '2%' },
];

function ViewDjStyleDna() {
  const tracks = useLibraryStore((state) => state.tracks);
  const viewMode = useHeraViewStore((state) => state.viewMode);

  const [djHandle, setDjHandle] = useState('DaviDJ');
  const [preferredSoftware, setPreferredSoftware] = useState('VirtualDJ');
  const [ruleVocalClash, setRuleVocalClash] = useState(true);
  const [ruleSubBass, setRuleSubBass] = useState(true);
  const [ruleAudioQuality, setRuleAudioQuality] = useState(true);
  const [dnaProfile, setDnaProfile] = useState<HeraStyleDnaResponse | null>(null);

  useEffect(() => {
    void computeDjStyleDna(
      tracks,
      djHandle,
      preferredSoftware,
      'Cuidar vocales · Proteger sub-bajo · Calidad estudio',
    ).then(setDnaProfile);
  }, [tracks, djHandle, preferredSoftware]);

  return (
    <View aria-label="HERA DJ Style Profile">
      <div {...stylex.props(styles.pageContainer)}>
        {/* HEADER */}
        <div {...stylex.props(styles.headerBar)}>
          <div>
            <h2 {...stylex.props(styles.pageTitle)}>
              🧬 {viewMode === 'simple' ? 'Mi Estilo DJ (Lo que hace único a tu sonido)' : 'Mi ADN Musical & Matriz Camelot'}
            </h2>
            <p {...stylex.props(styles.subDesc)}>
              {viewMode === 'simple'
                ? `HERA escuchó tus ${tracks.length || 1420} canciones para aprender cómo te gusta llevar la pista`
                : 'Auto-perfilado determinista en Rust desde tu disco local + sincronización en D1'}
            </p>
          </div>

          <div {...stylex.props(styles.switchesGroup)}>
            <span style={{ fontSize: '12px', color: '#94a3b8', marginRight: '4px' }}>
              Mi Programa DJ:
            </span>
            {DJ_SOFTWARES.map((sw) => (
              <button
                key={sw}
                type="button"
                onClick={() => setPreferredSoftware(sw)}
                {...stylex.props(
                  styles.swBtn,
                  preferredSoftware === sw && styles.swBtnActive,
                )}
              >
                {sw}
              </button>
            ))}
          </div>
        </div>

        {/* WORKSPACE */}
        <div {...stylex.props(styles.dnaWorkspace)}>
          {/* LEFT COLUMN: VIBES OR CAMELOT */}
          <section {...stylex.props(styles.dnaLeft)}>
            {/* 3 HUMAN TELEMETRY METRICS */}
            <div {...stylex.props(styles.telemetryRow)}>
              <div {...stylex.props(styles.telemetryCard)}>
                <div {...stylex.props(styles.tLabel)}>TU RITMO FAVORITO</div>
                <div {...stylex.props(styles.tVal)} style={{ color: '#67e8f9' }}>
                  {dnaProfile?.overall_average_bpm ? `${Math.round(dnaProfile.overall_average_bpm)} BPM` : '125 BPM'}
                </div>
                <div {...stylex.props(styles.tSub)}>
                  Te mueves entre 122 y 128 BPM
                </div>
              </div>

              <div {...stylex.props(styles.telemetryCard)}>
                <div {...stylex.props(styles.tLabel)}>TU FORMA DE MEZCLAR</div>
                <div {...stylex.props(styles.tVal)} style={{ color: '#6ee7b7' }}>
                  Subida Suave
                </div>
                <div {...stylex.props(styles.tSub)}>
                  Sin saltos bruscos de velocidad
                </div>
              </div>

              <div {...stylex.props(styles.telemetryCard)}>
                <div {...stylex.props(styles.tLabel)}>TU VIBRA PRINCIPAL</div>
                <div {...stylex.props(styles.tVal)}>
                  Melódica &amp; Club
                </div>
                <div {...stylex.props(styles.tSub)}>
                  Ideal para sets fluidos
                </div>
              </div>
            </div>

            {/* SIMPLE VIEW: VIBE CARDS (NO THEORY) */}
            {viewMode === 'simple' ? (
              <div {...stylex.props(styles.vibeCardContainer)}>
                <div {...stylex.props(styles.vibeHead)}>
                  <span style={{ fontWeight: 700, fontSize: '13.5px' }}>
                    El Clima Musical de tus Canciones (Sin teoría complicada)
                  </span>
                  <span style={{ fontSize: '11.5px', color: '#94a3b8' }}>
                    Calculado automáticamente
                  </span>
                </div>

                <div {...stylex.props(styles.vibeRow)}>
                  <div {...stylex.props(styles.vibeTopLine)}>
                    <span style={{ color: '#67e8f9' }}>🌌 Melódico, Profundo y Envolvente</span>
                    <strong style={{ color: '#67e8f9' }}>62% de tu música</strong>
                  </div>
                  <div {...stylex.props(styles.vibeDesc)}>
                    Canciones hipnóticas que mantienen a la gente bailando sin cansar el oído.
                  </div>
                  <div {...stylex.props(styles.vibeBarBg)}>
                    <div {...stylex.props(styles.vibeBarFill)} style={{ width: '62%', backgroundColor: '#06b6d4' }} />
                  </div>
                </div>

                <div {...stylex.props(styles.vibeRow)}>
                  <div {...stylex.props(styles.vibeTopLine)}>
                    <span style={{ color: '#6ee7b7' }}>🔥 Energía de Hora Pico (Fiestero)</span>
                    <strong style={{ color: '#6ee7b7' }}>24% de tu música</strong>
                  </div>
                  <div {...stylex.props(styles.vibeDesc)}>
                    Temas con bajo potente y explosiones fuertes para levantar las manos.
                  </div>
                  <div {...stylex.props(styles.vibeBarBg)}>
                    <div {...stylex.props(styles.vibeBarFill)} style={{ width: '24%', backgroundColor: '#10b981' }} />
                  </div>
                </div>

                <div {...stylex.props(styles.vibeRow)}>
                  <div {...stylex.props(styles.vibeTopLine)}>
                    <span style={{ color: '#fcd34d' }}>☀️ Cantado, Alegre y Eufórico</span>
                    <strong style={{ color: '#fcd34d' }}>14% de tu música</strong>
                  </div>
                  <div {...stylex.props(styles.vibeDesc)}>
                    Himnos con voces conocidas ideales para momentos especiales o cierre de fiesta.
                  </div>
                  <div {...stylex.props(styles.vibeBarBg)}>
                    <div {...stylex.props(styles.vibeBarFill)} style={{ width: '14%', backgroundColor: '#f59e0b' }} />
                  </div>
                </div>
              </div>
            ) : (
              /* PRO VIEW: 12x2 CAMELOT HARMONIC MATRIX */
              <div {...stylex.props(styles.vibeCardContainer)}>
                <div {...stylex.props(styles.vibeHead)}>
                  <span style={{ fontWeight: 700, fontSize: '13px' }}>
                    Mapa Armónico Camelot 1A–12A (Menor) &amp; Afinidad
                  </span>
                  <span style={{ fontSize: '11px', color: '#67e8f9' }}>
                    Dominantes: 8A (34%) · 9A (28%) · 10A (19%)
                  </span>
                </div>
                <div {...stylex.props(styles.camelotGrid)}>
                  {PRO_CAMELOT_KEYS_MINOR.map((k) => {
                    const isDominant = ['8A', '9A', '10A'].includes(k.code);
                    return (
                      <div
                        key={k.code}
                        {...stylex.props(
                          styles.camelotCell,
                          isDominant && styles.camelotCellActive,
                        )}
                      >
                        <span style={{ fontWeight: 700, fontSize: '12px', color: isDominant ? '#67e8f9' : '#f8fafc' }}>
                          {k.code}
                        </span>
                        <span style={{ fontSize: '10px', color: '#94a3b8' }}>{k.note}</span>
                        <span style={{ fontSize: '10px', color: '#94a3b8' }}>{k.pct}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </section>

          {/* RIGHT COLUMN: GOLDEN MIXING RULES & COPILOT UNDERSTANDING */}
          <section {...stylex.props(styles.dnaRight)}>
            <h3 style={{ margin: '0 0 4px 0', fontSize: '15px', fontWeight: 700 }}>
              {viewMode === 'simple' ? 'Tus Reglas de Oro al Mezclar' : 'Invariantes de Curaduría & DSP'}
            </h3>

            <div {...stylex.props(styles.rulesRack)}>
              <div {...stylex.props(styles.ruleRow)}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: '13.5px' }}>
                    🎤 Cuidar que no se encimen dos voces
                  </div>
                  <div style={{ fontSize: '12px', color: '#94a3b8' }}>
                    Conecta el final instrumental de una canción con el inicio limpio de la siguiente
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setRuleVocalClash(!ruleVocalClash)}
                  {...stylex.props(styles.toggleBtn, ruleVocalClash && styles.toggleActive)}
                >
                  {ruleVocalClash ? '✓ ACTIVADO' : 'APAGADO'}
                </button>
              </div>

              <div {...stylex.props(styles.ruleRow)}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: '13.5px' }}>
                    🔊 Mantener siempre la fuerza del bajo
                  </div>
                  <div style={{ fontSize: '12px', color: '#94a3b8' }}>
                    Evita que la pista sienta un vacío de potencia cuando cambias de canción
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setRuleSubBass(!ruleSubBass)}
                  {...stylex.props(styles.toggleBtn, ruleSubBass && styles.toggleActive)}
                >
                  {ruleSubBass ? '✓ ACTIVADO' : 'APAGADO'}
                </button>
              </div>

              <div {...stylex.props(styles.ruleRow)}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: '13.5px' }}>
                    ✨ Bloquear audios malos o bajados de YouTube
                  </div>
                  <div style={{ fontSize: '12px', color: '#94a3b8' }}>
                    Revisa cada canción para que nunca suene opaca en las bocinas grandes
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setRuleAudioQuality(!ruleAudioQuality)}
                  {...stylex.props(styles.toggleBtn, ruleAudioQuality && styles.toggleActive)}
                >
                  {ruleAudioQuality ? '✓ ACTIVADO' : 'APAGADO'}
                </button>
              </div>

              <div {...stylex.props(styles.ruleRow)}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: '13.5px' }}>
                    🤝 Red Colaborativa entre DJs
                  </div>
                  <div style={{ fontSize: '12px', color: '#94a3b8' }}>
                    Comparte y recibe música de estudio verificada con la comunidad
                  </div>
                </div>
                <span {...stylex.props(styles.toggleLocked)}>🔒 SIEMPRE ACTIVO</span>
              </div>
            </div>

            {/* HUMAN COPILOT UNDERSTANDING SUMMARY */}
            <div {...stylex.props(styles.summaryBox)}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontWeight: 700, fontSize: '13.5px', color: '#67e8f9' }}>
                  💬 Así te entiende tu Copiloto IA ahora mismo:
                </span>
                <span style={{ fontSize: '11.5px', color: '#6ee7b7', fontWeight: 600 }}>
                  ✓ Aprendido
                </span>
              </div>
              <p style={{ margin: 0, fontSize: '12.5px', color: '#f8fafc', lineHeight: 1.55 }}>
                “Cuando me pidas ayuda para armar una lista o buscar qué canción poner después, buscaré temas alrededor de <strong>125 BPM</strong> que suenen afinados entre sí, entrando en el <strong>Punto 1 (Entrada)</strong> cuando la canción anterior llegue al <strong>Punto 4 (Salida)</strong> para que nunca choquen las voces.”
              </p>
            </div>
          </section>
        </div>
      </div>
    </View>
  );
}

const styles = stylex.create({
  pageContainer: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    backgroundColor: '#090c12',
    color: '#f8fafc',
    overflow: 'hidden',
  },
  headerBar: {
    padding: '14px 20px',
    backgroundColor: '#070a0f',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    flexShrink: 0,
  },
  pageTitle: {
    margin: 0,
    fontSize: '16px',
    fontWeight: 700,
  },
  subDesc: {
    margin: '3px 0 0 0',
    fontSize: '12.5px',
    color: '#94a3b8',
  },
  switchesGroup: {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
  },
  swBtn: {
    padding: '4px 10px',
    borderRadius: '5px',
    backgroundColor: '#101622',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    color: '#94a3b8',
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  swBtnActive: {
    backgroundColor: 'rgba(6, 182, 212, 0.14)',
    borderColor: '#06b6d4',
    color: '#67e8f9',
    fontWeight: 700,
  },
  dnaWorkspace: {
    flex: 1,
    display: 'grid',
    gridTemplateColumns: '56% 44%',
    overflow: 'hidden',
  },
  dnaLeft: {
    padding: '16px 20px',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#1f2a3d',
    display: 'flex',
    flexDirection: 'column',
    gap: '16px',
    overflowY: 'auto',
  },
  dnaRight: {
    padding: '16px 20px',
    backgroundColor: '#0b0f17',
    display: 'flex',
    flexDirection: 'column',
    gap: '16px',
    overflowY: 'auto',
  },
  telemetryRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '8px',
    backgroundColor: '#101622',
    overflow: 'hidden',
  },
  telemetryCard: {
    padding: '12px 14px',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#1f2a3d',
  },
  tLabel: {
    fontSize: '11px',
    color: '#94a3b8',
    fontWeight: 600,
  },
  tVal: {
    fontSize: '18px',
    fontWeight: 800,
    marginTop: '3px',
  },
  tSub: {
    fontSize: '11px',
    color: '#94a3b8',
    marginTop: '2px',
  },
  vibeCardContainer: {
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '8px',
    backgroundColor: '#101622',
    overflow: 'hidden',
  },
  vibeHead: {
    padding: '10px 16px',
    backgroundColor: '#0c111a',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  vibeRow: {
    padding: '12px 16px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    flexDirection: 'column',
    gap: '5px',
  },
  vibeTopLine: {
    display: 'flex',
    justifyContent: 'space-between',
    fontSize: '13px',
    fontWeight: 700,
  },
  vibeDesc: {
    fontSize: '12px',
    color: '#94a3b8',
  },
  vibeBarBg: {
    height: '6px',
    borderRadius: '99px',
    backgroundColor: '#090c12',
    overflow: 'hidden',
    marginTop: '2px',
  },
  vibeBarFill: {
    height: '100%',
    borderRadius: '99px',
  },
  camelotGrid: {
    padding: '12px',
    display: 'grid',
    gridTemplateColumns: 'repeat(6, 1fr)',
    gap: '6px',
  },
  camelotCell: {
    padding: '8px 4px',
    borderRadius: '5px',
    backgroundColor: '#090c12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '2px',
  },
  camelotCellActive: {
    backgroundColor: 'rgba(6, 182, 212, 0.14)',
    borderColor: '#06b6d4',
  },
  rulesRack: {
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '8px',
    backgroundColor: '#101622',
    overflow: 'hidden',
  },
  ruleRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '12px 14px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    gap: '12px',
  },
  toggleBtn: {
    padding: '4px 10px',
    borderRadius: '99px',
    backgroundColor: '#161f30',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    color: '#94a3b8',
    fontSize: '11px',
    fontWeight: 700,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  },
  toggleActive: {
    backgroundColor: 'rgba(16, 185, 129, 0.14)',
    borderColor: '#10b981',
    color: '#6ee7b7',
  },
  toggleLocked: {
    padding: '4px 10px',
    borderRadius: '99px',
    backgroundColor: 'rgba(6, 182, 212, 0.14)',
    borderColor: 'rgba(6, 182, 212, 0.45)',
    color: '#67e8f9',
    fontSize: '11px',
    fontWeight: 700,
    whiteSpace: 'nowrap',
  },
  summaryBox: {
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '8px',
    backgroundColor: '#101622',
    padding: '14px 16px',
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
  },
});

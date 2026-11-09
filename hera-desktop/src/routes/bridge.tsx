import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

import View from '../elements/View';
import {
  chatWithCloudflareFreeAi,
  downloadP2PTrackToLibrary,
  getCloudflareEdgeRadar,
  probeLocalAiBackend,
  resolveHarmonicBridge,
  scanAiConnectivityOnboarding,
  type AiConnectivityOnboardingReport,
  type CloudflareEdgeCapabilities,
  type CommanderAuthorityMode,
  type HeraCloudflareFreeAiResponse,
  type ResolveHarmonicBridgeResponse,
} from '../lib/bridge-hera';
import {
  getHumanAudioQuality,
  getHumanCueInfo,
  getHumanMatchLabel,
  useHeraViewStore,
} from '../lib/hera-human-language';
import Player from '../lib/player';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/bridge')({
  component: ViewHarmonicBridgeP2P,
});

type SwarmSuperFilter = 'all' | 'harmonic' | 'boost' | 'promo';

function ViewHarmonicBridgeP2P() {
  const localTracks = useLibraryStore((state) => state.tracks);
  const viewMode = useHeraViewStore((state) => state.viewMode);
  const isOnboardingOpen = useHeraViewStore((state) => state.isOnboardingOpen);
  const closeOnboarding = useHeraViewStore((state) => state.closeOnboarding);
  const openOnboarding = useHeraViewStore((state) => state.openOnboarding);

  const [swarmFilter, setSwarmFilter] = useState('');
  const [superFilter, setSuperFilter] = useState<SwarmSuperFilter>('all');
  const [originTitle, setOriginTitle] = useState('Levels (Original Mix)');
  const [originArtist, setOriginArtist] = useState('Avicii');
  const [originBpm, setOriginBpm] = useState(124.0);
  const [originCamelot, setOriginCamelot] = useState('8A');

  const [targetTitle, setTargetTitle] = useState("Don't You Worry Child");
  const [targetArtist, setTargetArtist] = useState('Swedish House Mafia');
  const [targetBpm, setTargetBpm] = useState(128.0);
  const [targetCamelot, setTargetCamelot] = useState('10A');

  const [bridgeResult, setBridgeResult] =
    useState<ResolveHarmonicBridgeResponse | null>(null);
  const [edgeRadar, setEdgeRadar] = useState<CloudflareEdgeCapabilities | null>(
    null,
  );
  const [previewStatus, setPreviewStatus] = useState<string>('');
  const [chatPrompt, setChatPrompt] = useState(
    'Quiero pasar de Levels a Don\'t You Worry Child subiendo el ritmo suave y sin que choquen las voces',
  );
  const [selectedBackend, setSelectedBackend] = useState<string>('cli_agy');
  const [customBaseUrl, setCustomBaseUrl] = useState<string>(
    'http://localhost:1234/v1',
  );
  const [customModel, setCustomModel] = useState<string>('default');
  const [probeBanner, setProbeBanner] = useState<string>('');
  const [commanderMode, setCommanderMode] =
    useState<CommanderAuthorityMode>('executive_boss');
  const [onboardingReport, setOnboardingReport] =
    useState<AiConnectivityOnboardingReport | null>(null);
  const [isScanningOnboarding, setIsScanningOnboarding] =
    useState<boolean>(false);
  const [aiResponse, setAiResponse] =
    useState<HeraCloudflareFreeAiResponse | null>(null);
  const [isAiLoading, setIsAiLoading] = useState<boolean>(false);
  const [showProOptions, setShowProOptions] = useState<boolean>(false);

  const handleRunOnboardingScan = async () => {
    setIsScanningOnboarding(true);
    try {
      const rep = await scanAiConnectivityOnboarding(commanderMode);
      setOnboardingReport(rep);
      if (rep.recommended_backend_id) {
        setSelectedBackend(rep.recommended_backend_id);
      }
      setProbeBanner(
        viewMode === 'simple'
          ? `✓ Copiloto listo: ${rep.recommended_backend_id.replace('cli_', '')} · Calidad y comunidad blindadas`
          : `🧭 ${rep.summary_badge} · Escudo del Enjambre: Quarantine-First Activo`,
      );
    } catch {
      // Fallback
    } finally {
      setIsScanningOnboarding(false);
    }
  };

  const handleSendCopilotCommand = async (customText?: string) => {
    const promptToSend = customText || chatPrompt;
    if (!promptToSend.trim()) return;

    setIsAiLoading(true);
    try {
      const reply = await chatWithCloudflareFreeAi(
        promptToSend,
        'DaviDJ',
        selectedBackend,
        commanderMode,
      );
      setAiResponse(reply);
      setPreviewStatus(
        viewMode === 'simple'
          ? '✨ Propuesta lista: revisa los pasos y dale a "Me gusta" para guardar'
          : `🤖 Respuesta generada por ${reply.model_used}`,
      );
    } catch {
      setPreviewStatus('⚠️ Conectando con el copiloto...');
    } finally {
      setIsAiLoading(false);
    }
  };

  useEffect(() => {
    const loadInitialData = async () => {
      try {
        const radar = await getCloudflareEdgeRadar();
        setEdgeRadar(radar);
      } catch {
        // Fallback
      }

      try {
        const res = await resolveHarmonicBridge(
          originTitle,
          originArtist,
          originBpm,
          originCamelot,
          targetTitle,
          targetArtist,
          targetBpm,
          targetCamelot,
        );
        setBridgeResult(res);
      } catch {
        // Fallback
      }

      handleRunOnboardingScan();
    };

    loadInitialData();
  }, []);

  const matchInfo = getHumanMatchLabel(
    originCamelot,
    targetCamelot,
    originBpm,
    targetBpm,
  );

  return (
    <View aria-label="HERA Studio & Copilot Console">
      <div {...stylex.props(styles.pageContainer)}>
        {/* TOP STATUS BAR */}
        <header {...stylex.props(styles.topBar)}>
          <div {...stylex.props(styles.brandGroup)}>
            <span {...stylex.props(styles.brandDot)} />
            <strong {...stylex.props(styles.brandTitle)}>HERA DJ</strong>
            <span {...stylex.props(styles.divider)} />
            <span {...stylex.props(styles.subLabel)}>
              {viewMode === 'simple'
                ? 'Conectar Canciones & Copiloto de Cabina'
                : 'Consola de Puente Armónico & Copiloto ReAct'}
            </span>
          </div>

          <div {...stylex.props(styles.topActions)}>
            <div
              {...stylex.props(styles.statusPill, styles.activePill)}
              onClick={openOnboarding}
              role="button"
              tabIndex={0}
              title="Abrir panel de copiloto e IA"
            >
              <span {...stylex.props(styles.greenDot)} />
              {viewMode === 'simple'
                ? `Copiloto: ${selectedBackend.replace('cli_', '')} · 👑 Tú eres El Jefe ▾`
                : `IA: ${selectedBackend} · Modo: ${commanderMode} ▾`}
            </div>

            <button
              type="button"
              onClick={openOnboarding}
              {...stylex.props(styles.statusPill)}
            >
              🔌 {viewMode === 'simple' ? 'Cambiar IA' : 'Radar IA (5 Modos)'}
            </button>

            <div {...stylex.props(styles.statusPill, styles.emeraldPill)}>
              🤝 {viewMode === 'simple' ? 'Red de DJs Conectada' : 'Enjambre P2P Seeding Activo'}
            </div>
          </div>
        </header>

        {/* MAIN SPLIT-SCREEN WORKSPACE */}
        <div {...stylex.props(styles.splitWorkspace)}>
          {/* LEFT COLUMN: BRIDGE DECKS, STEPS & COMMUNITY TRACKS */}
          <section {...stylex.props(styles.leftCol)}>
            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Puente Musical (Cómo pasar de una canción a otra sin que choquen)'
                  : 'Puente Armónico 2-Opt TSP (Deck A ➔ Deck B)'}
              </h2>
              <span {...stylex.props(styles.matchTag)}>
                {viewMode === 'simple'
                  ? `${matchInfo.compatibilityLabel} · ${matchInfo.tempoAdvice}`
                  : `Delta: ${(targetBpm - originBpm).toFixed(1)} BPM · ${originCamelot} ➔ ${targetCamelot}`}
              </span>
            </div>

            {/* DECKS A & B */}
            <div {...stylex.props(styles.deckRack)}>
              <div {...stylex.props(styles.decksRow)}>
                <div {...stylex.props(styles.deckCard)}>
                  <div {...stylex.props(styles.deckTopLine)}>
                    <span {...stylex.props(styles.deckLabel)}>
                      {viewMode === 'simple' ? '💿 SONANDO AHORA' : 'DECK A · ORIGEN'}
                    </span>
                    <span {...stylex.props(styles.badgeCyan)}>
                      {viewMode === 'simple'
                        ? `Melódica · ${originBpm} BPM`
                        : `${originCamelot} · ${originBpm} BPM`}
                    </span>
                  </div>
                  <div {...stylex.props(styles.deckSongTitle)}>
                    {originArtist} — {originTitle}
                  </div>
                  <div {...stylex.props(styles.deckDetails)}>
                    <span>✨ Sonido de Estudio</span>
                    <span>•</span>
                    <span>Salida recomendada: <strong>04:42</strong></span>
                  </div>
                </div>

                <div {...stylex.props(styles.arrowCol)}>➔</div>

                <div {...stylex.props(styles.deckCard)}>
                  <div {...stylex.props(styles.deckTopLine)}>
                    <span {...stylex.props(styles.deckLabel)}>
                      {viewMode === 'simple' ? '🎯 QUIERO LLEGAR A' : 'DECK B · DESTINO'}
                    </span>
                    <span {...stylex.props(styles.badgeEmerald)}>
                      {viewMode === 'simple'
                        ? `Eufórica · ${targetBpm} BPM`
                        : `${targetCamelot} · ${targetBpm} BPM`}
                    </span>
                  </div>
                  <div {...stylex.props(styles.deckSongTitle)}>
                    {targetArtist} — {targetTitle}
                  </div>
                  <div {...stylex.props(styles.deckDetails)}>
                    <span>✨ Sonido de Estudio</span>
                    <span>•</span>
                    <span>Entrada recomendada: <strong>00:15</strong></span>
                  </div>
                </div>
              </div>

              {/* INTERMEDIATE BRIDGE STEPS */}
              <div {...stylex.props(styles.stepsContainer)}>
                <div {...stylex.props(styles.stepRow)}>
                  <span {...stylex.props(styles.stepIdx)}>PASO 1</span>
                  <div {...stylex.props(styles.stepName)}>
                    Eric Prydz — Opus{' '}
                    <span {...stylex.props(styles.dimText)}>
                      {viewMode === 'simple'
                        ? '(Sube el ritmo suave)'
                        : '(8A ➔ 9A)'}
                    </span>
                  </div>
                  <span {...stylex.props(styles.matchTag)}>💚 Combina 98%</span>
                  <span {...stylex.props(styles.bpmTag)}>125.5 BPM</span>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus('🎧 Reproduciendo cruce de 15s (Salida ➔ Entrada)...')
                    }
                    {...stylex.props(styles.previewBtn)}
                  >
                    🎧 {viewMode === 'simple' ? 'Oír cómo mezcla (15s)' : 'Cruce 15s (4➔1)'}
                  </button>
                </div>

                <div {...stylex.props(styles.stepRow)}>
                  <span {...stylex.props(styles.stepIdx)}>PASO 2</span>
                  <div {...stylex.props(styles.stepName)}>
                    Deadmau5 — Strobe{' '}
                    <span {...stylex.props(styles.dimText)}>
                      {viewMode === 'simple'
                        ? '(Prepara la entrada del coro)'
                        : '(9A ➔ 10A)'}
                    </span>
                  </div>
                  <span {...stylex.props(styles.matchTag)}>💚 Combina 97%</span>
                  <span {...stylex.props(styles.bpmTag)}>127.0 BPM</span>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus('🎧 Reproduciendo cruce de 15s (Salida ➔ Entrada)...')
                    }
                    {...stylex.props(styles.previewBtn)}
                  >
                    🎧 {viewMode === 'simple' ? 'Oír cómo mezcla (15s)' : 'Cruce 15s (4➔1)'}
                  </button>
                </div>
              </div>

              {/* 4 HOT CUES STRIP */}
              <div {...stylex.props(styles.cueStrip)}>
                <span {...stylex.props(styles.cueStripTitle)}>
                  {viewMode === 'simple'
                    ? 'PUNTOS CLAVE PARA MEZCLAR:'
                    : 'HOT CUES ACÚSTICOS DSP:'}
                </span>
                <div {...stylex.props(styles.cuePinsList)}>
                  <span {...stylex.props(styles.cuePin)}>
                    <span {...stylex.props(styles.dot, styles.dotGreen)} />
                    {viewMode === 'simple' ? '1: Entrada (0:16)' : '1: Intro (00:16)'}
                  </span>
                  <span {...stylex.props(styles.cuePin)}>
                    <span {...stylex.props(styles.dot, styles.dotBlue)} />
                    {viewMode === 'simple' ? '2: Calma (1:48)' : '2: Break (01:48)'}
                  </span>
                  <span {...stylex.props(styles.cuePin)}>
                    <span {...stylex.props(styles.dot, styles.dotRed)} />
                    {viewMode === 'simple' ? '3: Explosión (2:34)' : '3: Drop (02:34)'}
                  </span>
                  <span {...stylex.props(styles.cuePin)}>
                    <span {...stylex.props(styles.dot, styles.dotYellow)} />
                    {viewMode === 'simple' ? '4: Salida limpia (5:12)' : '4: Outro (05:12)'}
                  </span>
                </div>
              </div>
            </div>

            {/* COMMUNITY / SWARM TRACKS TABLE */}
            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Otras canciones de la Comunidad que quedan increíbles en esta mezcla'
                  : 'Índice Federado del Enjambre P2P (Candidatos Armónicos)'}
              </h2>
              <span {...stylex.props(styles.dimText)}>
                ✓ {viewMode === 'simple' ? 'Todas con sonido de estudio' : 'Cuarentena FFT > 20 kHz'}
              </span>
            </div>

            <div {...stylex.props(styles.tableWrap)}>
              <table {...stylex.props(styles.table)}>
                <thead>
                  <tr>
                    <th>CANCIÓN / ARTISTA</th>
                    <th>¿CÓMO QUEDA?</th>
                    <th>RITMO</th>
                    <th>CALIDAD</th>
                    <th>ACCIÓN</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>
                      <strong>Pryda — Allein</strong>{' '}
                      <span {...stylex.props(styles.dimText)}>· Original Mix</span>
                    </td>
                    <td>
                      <span {...stylex.props(styles.matchTag)}>💚 Mezcla suave · Sin voces</span>
                    </td>
                    <td>126.0 BPM</td>
                    <td style={{ color: '#6ee7b7', fontWeight: 600 }}>✨ Estudio (Puro)</td>
                    <td>
                      <button
                        type="button"
                        onClick={() =>
                          setPreviewStatus('📥 Agregado "Pryda — Allein" a tu lista de prueba')
                        }
                        {...stylex.props(styles.tableBtn)}
                      >
                        ➕ Probar
                      </button>
                    </td>
                  </tr>
                  <tr>
                    <td>
                      <strong>RÜFÜS DU SOL — Innerbloom</strong>{' '}
                      <span {...stylex.props(styles.dimText)}>· Lane 8 Remix</span>
                    </td>
                    <td>
                      <span {...stylex.props(styles.matchTag)}>💚 Mantiene la vibra melódica</span>
                    </td>
                    <td>124.5 BPM</td>
                    <td style={{ color: '#6ee7b7', fontWeight: 600 }}>✨ Estudio (Puro)</td>
                    <td>
                      <button
                        type="button"
                        onClick={() =>
                          setPreviewStatus('📥 Agregado "Innerbloom" a tu lista de prueba')
                        }
                        {...stylex.props(styles.tableBtn)}
                      >
                        ➕ Probar
                      </button>
                    </td>
                  </tr>
                  <tr>
                    <td>
                      <strong>ARTBAT — Flame</strong>{' '}
                      <span {...stylex.props(styles.dimText)}>· Extended Mix</span>
                    </td>
                    <td>
                      <span {...stylex.props(styles.vibeTag)}>🔥 Levanta la pista</span>
                    </td>
                    <td>126.0 BPM</td>
                    <td style={{ color: '#6ee7b7', fontWeight: 600 }}>✨ Estudio (Puro)</td>
                    <td>
                      <button
                        type="button"
                        onClick={() =>
                          setPreviewStatus('📥 Agregado "ARTBAT — Flame" a tu lista de prueba')
                        }
                        {...stylex.props(styles.tableBtn)}
                      >
                        ➕ Probar
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* RIGHT COLUMN: EXECUTIVE COPILOT DOCK */}
          <section {...stylex.props(styles.rightCol)}>
            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Tu Copiloto de Cabina (Háblale como a otro DJ)'
                  : 'Consola Ejecutiva "El Jefe" · Copiloto ReAct'}
              </h2>
              <span style={{ color: '#67e8f9', fontSize: '12px', fontWeight: 600 }}>
                {viewMode === 'simple' ? 'Gratis · Tú apruebas cada cambio' : `${selectedBackend} · $0.00`}
              </span>
            </div>

            <div {...stylex.props(styles.copilotBody)}>
              {/* BOSS MESSAGE */}
              <div {...stylex.props(styles.bossMsg)}>
                <div {...stylex.props(styles.bossHeader)}>
                  <span>👑 TÚ (EL JEFE)</span>
                  <span>Hace 1 min</span>
                </div>
                Quiero pasar de "Levels" a "Don't You Worry Child" subiendo el ánimo poco a poco, pero que no se encimen las voces al mezclar.
              </div>

              {/* RECEIPT / ANSWER PANEL */}
              <div {...stylex.props(styles.receiptBox)}>
                <div {...stylex.props(styles.receiptHead)}>
                  <div {...stylex.props(styles.receiptTitle)}>
                    <span {...stylex.props(styles.greenDot)} />
                    {viewMode === 'simple'
                      ? '¡LISTO JEFE! TE ARMÉ ESTE PUENTE DE 2 CANCIONES'
                      : 'PROPUESTA DE INGENIERÍA 2-OPT TSP LISTA'}
                  </div>
                  <span style={{ fontSize: '11px', color: '#94a3b8' }}>
                    {viewMode === 'simple' ? 'Sonido Verificado' : '18ms · 0 Tokens Extra'}
                  </span>
                </div>

                <div {...stylex.props(styles.receiptBody)}>
                  <p style={{ margin: 0 }}>
                    {viewMode === 'simple'
                      ? 'Para no saltar de golpe de 124 a 128 BPM, puse dos temas en medio que suben la energía de forma natural y suenan afinados entre sí:'
                      : 'Curva calculada con incremento gradual de +1.5 BPM por paso y compatibilidad armónica > 96%:'}
                  </p>

                  <div {...stylex.props(styles.metricsRow)}>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricLabel)}>SUBIDA DE RITMO</div>
                      <div {...stylex.props(styles.metricVal)} style={{ color: '#67e8f9' }}>
                        Suave (+1.5 BPM)
                      </div>
                    </div>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricLabel)}>CUIDADO DE VOCES</div>
                      <div {...stylex.props(styles.metricVal)} style={{ color: '#6ee7b7' }}>
                        🎤 Cero Choques
                      </div>
                    </div>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricLabel)}>CALIDAD DE SONIDO</div>
                      <div {...stylex.props(styles.metricVal)} style={{ color: '#6ee7b7' }}>
                        ✨ 100% Estudio
                      </div>
                    </div>
                  </div>

                  <div {...stylex.props(styles.humanInstructions)}>
                    <div><strong>1. Pon "Eric Prydz — Opus"</strong> cuando empiece la salida de Levels (min 04:42).</div>
                    <div><strong>2. Mezcla "Deadmau5 — Strobe"</strong> en el minuto 05:12 (pura base instrumental, sin voz).</div>
                    <div><strong>3. Suelta "Don't You Worry Child"</strong> y el coro entrará con toda la fuerza a 128 BPM.</div>
                  </div>
                </div>

                <div {...stylex.props(styles.receiptActions)}>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus('✅ Puente aprobado y promovido a tu lista de cabina')
                    }
                    {...stylex.props(styles.primaryBtn)}
                  >
                    ✅ {viewMode === 'simple' ? 'Me gusta, agregar a mi Lista' : 'Aprobar y Promover'}
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus('🎛️ Exportando directamente a tu programa DJ...')
                    }
                    {...stylex.props(styles.secondaryBtn)}
                  >
                    🎛️ {viewMode === 'simple' ? 'Mandar a VirtualDJ' : 'Exportar a VirtualDJ'}
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus('🎧 Reproduciendo simulación completa del cruce...')
                    }
                    {...stylex.props(styles.secondaryBtn)}
                  >
                    🎧 Escuchar Mezcla
                  </button>
                </div>
              </div>

              {previewStatus && (
                <div {...stylex.props(styles.statusAlert)}>{previewStatus}</div>
              )}
            </div>

            {/* COPILOT INPUT BAR */}
            <div {...stylex.props(styles.inputDock)}>
              <div {...stylex.props(styles.chipsList)}>
                <button
                  type="button"
                  onClick={() =>
                    handleSendCopilotCommand('Pon algo más fiestero para levantar la pista')
                  }
                  {...stylex.props(styles.chip)}
                >
                  🔥 Pon algo más fiestero
                </button>
                <button
                  type="button"
                  onClick={() =>
                    handleSendCopilotCommand('Baja un poco la velocidad para ir más suave')
                  }
                  {...stylex.props(styles.chip)}
                >
                  🌙 Baja un poco la velocidad
                </button>
                <button
                  type="button"
                  onClick={() =>
                    handleSendCopilotCommand('Busca temas que no tengan voces al principio')
                  }
                  {...stylex.props(styles.chip)}
                >
                  🎤 Busca temas sin voz al inicio
                </button>
              </div>

              <div {...stylex.props(styles.inputRow)}>
                <input
                  type="text"
                  value={chatPrompt}
                  onChange={(e) => setChatPrompt(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleSendCopilotCommand();
                  }}
                  placeholder={
                    viewMode === 'simple'
                      ? 'Pídele lo que quieras (ej. "Cambia la segunda canción por algo más movido")...'
                      : 'Ordena al copiloto (ej. "Sustituye el paso 2 por Pryda - Allein")...'
                  }
                  {...stylex.props(styles.chatInput)}
                />
                <button
                  type="button"
                  onClick={() => handleSendCopilotCommand()}
                  disabled={isAiLoading}
                  {...stylex.props(styles.primaryBtn)}
                >
                  {isAiLoading ? 'Pensando...' : 'Pedir al Copiloto'}
                </button>
              </div>
            </div>
          </section>
        </div>

        {/* ONBOARDING DRAWER (CONTEXTUAL SLIDE-OUT) */}
        {isOnboardingOpen && (
          <div
            {...stylex.props(styles.drawerOverlay)}
            onClick={(e) => {
              if (e.target === e.currentTarget) closeOnboarding();
            }}
          >
            <aside {...stylex.props(styles.drawer)}>
              <div {...stylex.props(styles.drawerHeader)}>
                <div>
                  <h2 style={{ margin: 0, fontSize: '16px', fontWeight: 700 }}>
                    {viewMode === 'simple'
                      ? 'Conectar tu Copiloto IA (Sin configuraciones técnicas)'
                      : 'Onboarding & Radar Universal de Conectividad IA'}
                  </h2>
                  <p style={{ margin: '4px 0 0 0', fontSize: '12px', color: '#94a3b8' }}>
                    {viewMode === 'simple'
                      ? 'Elige cómo quieres que te ayude HERA. Funciona gratis desde el primer segundo.'
                      : 'Detecta y activa los 5 modos de IA disponibles en tu sistema.'}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={closeOnboarding}
                  {...stylex.props(styles.closeBtn)}
                >
                  Cerrar ✕
                </button>
              </div>

              <div {...stylex.props(styles.drawerBody)}>
                {/* 1. AUTHORITY MODE */}
                <div>
                  <div {...stylex.props(styles.subHeadRow)}>
                    <strong>1. ¿Quién toma las decisiones en la cabina?</strong>
                    <span style={{ color: '#67e8f9', fontSize: '12px', fontWeight: 600 }}>
                      {commanderMode === 'executive_boss'
                        ? '👑 Tú eres El Jefe'
                        : commanderMode === 'autonomous_operator'
                        ? '⚡ Piloto Automático'
                        : '👀 Solo Consejos'}
                    </span>
                  </div>

                  <div {...stylex.props(styles.authGrid)}>
                    <div
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'executive_boss' && styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('executive_boss')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        👑 Tú eres El Jefe <span>{commanderMode === 'executive_boss' && '●'}</span>
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        El Copiloto te sugiere mezclas y canciones, pero tú apruebas con 1 clic antes de guardar.
                      </div>
                    </div>

                    <div
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'autonomous_operator' && styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('autonomous_operator')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        ⚡ Piloto Automático <span>{commanderMode === 'autonomous_operator' && '●'}</span>
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        El Copiloto arma, ordena y deja listas tus carpetas automáticamente respetando la calidad.
                      </div>
                    </div>

                    <div
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'advisor_only' && styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('advisor_only')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        👀 Solo Consejos <span>{commanderMode === 'advisor_only' && '●'}</span>
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        Solo te dice qué canciones combinan bien sin descargar ni mover nada.
                      </div>
                    </div>
                  </div>
                </div>

                {/* 2. SIMPLE AI OPTIONS */}
                <div>
                  <div {...stylex.props(styles.subHeadRow)}>
                    <strong>2. Elige el cerebro de tu Copiloto (Todos a $0.00 extra)</strong>
                    <span style={{ color: '#6ee7b7', fontSize: '12px', fontWeight: 600 }}>
                      ✓ Listo para usar
                    </span>
                  </div>

                  <div {...stylex.props(styles.modesContainer)}>
                    {/* LOCAL CLI APPS */}
                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend.startsWith('cli_') && styles.modeRowActive,
                      )}
                      onClick={() => setSelectedBackend('cli_agy')}
                    >
                      <div>
                        <div {...stylex.props(styles.modeTitle)}>
                          ⚡ Usar la IA que ya tienes en tu computadora
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          Detectamos estas apps instaladas en tu PC. Úsalas directo sin pagar ni configurar nada extra:
                        </div>
                        <div {...stylex.props(styles.badgesRow)}>
                          <span {...stylex.props(styles.pillActiveBadge)}>
                            ✓ Antigravity ({selectedBackend === 'cli_agy' ? 'Activo' : 'Detectado'})
                          </span>
                          <span {...stylex.props(styles.pillBadge)}>✓ Claude (Detectado)</span>
                          <span {...stylex.props(styles.pillBadge)}>✓ Gemini (Detectado)</span>
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelectedBackend('cli_agy')}
                        {...stylex.props(
                          selectedBackend === 'cli_agy'
                            ? styles.primaryBtn
                            : styles.secondaryBtn,
                        )}
                      >
                        {selectedBackend === 'cli_agy' ? '✓ Activo' : 'Usar'}
                      </button>
                    </div>

                    {/* FREE CLOUD */}
                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend === 'workers_ai_llama33' && styles.modeRowActive,
                      )}
                      onClick={() => setSelectedBackend('workers_ai_llama33')}
                    >
                      <div>
                        <div {...stylex.props(styles.modeTitle)}>
                          ☁️ IA Gratuita de HERA (Incluida para todos)
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          Funciona al instante por internet sin instalar programas ni crear cuentas (10,000 Neurons/día gratis).
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelectedBackend('workers_ai_llama33')}
                        {...stylex.props(
                          selectedBackend === 'workers_ai_llama33'
                            ? styles.primaryBtn
                            : styles.secondaryBtn,
                        )}
                      >
                        {selectedBackend === 'workers_ai_llama33' ? '✓ Activo' : 'Usar Gratis'}
                      </button>
                    </div>

                    {/* OFFLINE CLUB */}
                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend === 'ollama' && styles.modeRowActive,
                      )}
                      onClick={() => setSelectedBackend('ollama')}
                    >
                      <div>
                        <div {...stylex.props(styles.modeTitle)}>
                          ✈️ Modo Fiesta Sin Internet (100% en tu PC)
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          Ideal cuando estás tocando en vivo en un club, evento o cabaña sin Wi-Fi (Ollama / LM Studio).
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelectedBackend('ollama')}
                        {...stylex.props(
                          selectedBackend === 'ollama'
                            ? styles.primaryBtn
                            : styles.secondaryBtn,
                        )}
                      >
                        {selectedBackend === 'ollama' ? '✓ Activo' : 'Activar'}
                      </button>
                    </div>

                    {/* COLLAPSIBLE PRO SECTION */}
                    <div {...stylex.props(styles.proExpanderRow)}>
                      <div style={{ fontSize: '12px', color: '#94a3b8' }}>
                        ⚙️ <strong>¿Eres usuario avanzado?</strong> También puedes conectar tu propia API key o usar HERA vía MCP.
                      </div>
                      <button
                        type="button"
                        onClick={() => setShowProOptions(!showProOptions)}
                        {...stylex.props(styles.secondaryBtn)}
                        style={{ height: '28px', fontSize: '11.5px' }}
                      >
                        {showProOptions ? 'Ocultar Opciones Pro ▴' : 'Ver Opciones Pro ▾'}
                      </button>
                    </div>

                    {showProOptions && (
                      <div {...stylex.props(styles.proBox)}>
                        <div style={{ fontSize: '12px', color: '#cbd5e1' }}>
                          🔌 <strong>HERA MCP Server:</strong> Ejecuta <code>hera-node --mcp</code> en Claude Desktop, Cursor o Antigravity.
                        </div>
                        <div style={{ display: 'flex', gap: '8px', marginTop: '6px' }}>
                          <input
                            type="text"
                            value={customBaseUrl}
                            onChange={(e) => setCustomBaseUrl(e.target.value)}
                            placeholder="URL Servidor Local (ej. http://localhost:1234/v1)"
                            {...stylex.props(styles.chatInput)}
                            style={{ height: '30px', fontSize: '12px' }}
                          />
                          <button
                            type="button"
                            onClick={() =>
                              setPreviewStatus('✓ Servidor local guardado correctamente')
                            }
                            {...stylex.props(styles.secondaryBtn)}
                            style={{ height: '30px', fontSize: '12px' }}
                          >
                            Guardar
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                {/* SWARM FIRST INVARIANT */}
                <div {...stylex.props(styles.swarmLockBar)}>
                  <div>
                    <div style={{ fontWeight: 700, fontSize: '13px', color: '#6ee7b7' }}>
                      🤝 Comunidad HERA: Todos nos ayudamos
                    </div>
                    <div style={{ fontSize: '11.5px', color: '#94a3b8' }}>
                      HERA es 100% gratis porque compartimos y cuidamos la calidad de audio entre todos los DJs.
                    </div>
                  </div>
                  <span {...stylex.props(styles.toggleLocked)}>🤝 SIEMPRE ACTIVO</span>
                </div>
              </div>
            </aside>
          </div>
        )}
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
  topBar: {
    height: '48px',
    backgroundColor: '#070a0f',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '0 16px',
    flexShrink: 0,
  },
  brandGroup: {
    display: 'flex',
    alignItems: 'center',
    gap: '10px',
  },
  brandDot: {
    width: '8px',
    height: '8px',
    borderRadius: '50%',
    backgroundColor: '#06b6d4',
    boxShadow: '0 0 8px #06b6d4',
  },
  brandTitle: {
    fontSize: '14px',
    letterSpacing: '0.04em',
    color: '#f8fafc',
  },
  divider: {
    width: '1px',
    height: '16px',
    backgroundColor: '#1f2a3d',
  },
  subLabel: {
    fontSize: '13px',
    color: '#94a3b8',
    fontWeight: 500,
  },
  topActions: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  },
  statusPill: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '6px',
    height: '28px',
    padding: '0 10px',
    borderRadius: '5px',
    backgroundColor: '#101622',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    fontSize: '12px',
    color: '#94a3b8',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  },
  activePill: {
    backgroundColor: 'rgba(6, 182, 212, 0.12)',
    borderColor: 'rgba(6, 182, 212, 0.4)',
    color: '#67e8f9',
    fontWeight: 600,
  },
  emeraldPill: {
    backgroundColor: 'rgba(16, 185, 129, 0.12)',
    borderColor: 'rgba(16, 185, 129, 0.35)',
    color: '#6ee7b7',
    fontWeight: 600,
  },
  greenDot: {
    width: '6px',
    height: '6px',
    borderRadius: '50%',
    backgroundColor: '#10b981',
  },
  splitWorkspace: {
    flex: 1,
    display: 'grid',
    gridTemplateColumns: '56% 44%',
    overflow: 'hidden',
  },
  leftCol: {
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#1f2a3d',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  rightCol: {
    display: 'flex',
    flexDirection: 'column',
    backgroundColor: '#0b0f17',
    overflow: 'hidden',
  },
  sectionHead: {
    height: '42px',
    padding: '0 16px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#101622',
    flexShrink: 0,
  },
  sectionTitle: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#f8fafc',
  },
  deckRack: {
    padding: '14px 16px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    flexDirection: 'column',
    gap: '12px',
  },
  decksRow: {
    display: 'grid',
    gridTemplateColumns: '1fr 32px 1fr',
    alignItems: 'center',
    gap: '10px',
  },
  deckCard: {
    backgroundColor: '#101622',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '6px',
    padding: '10px 14px',
    display: 'flex',
    flexDirection: 'column',
    gap: '4px',
  },
  deckTopLine: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  deckLabel: {
    fontSize: '11px',
    fontWeight: 700,
    color: '#67e8f9',
  },
  deckSongTitle: {
    fontSize: '14px',
    fontWeight: 700,
    color: '#f8fafc',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  deckDetails: {
    fontSize: '12px',
    color: '#94a3b8',
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  },
  arrowCol: {
    textAlign: 'center',
    color: '#06b6d4',
    fontSize: '16px',
    fontWeight: 800,
  },
  stepsContainer: {
    backgroundColor: '#101622',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '6px',
    overflow: 'hidden',
  },
  stepRow: {
    display: 'grid',
    gridTemplateColumns: '70px 1fr 130px 80px 160px',
    alignItems: 'center',
    padding: '8px 12px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    gap: '8px',
    fontSize: '12.5px',
  },
  stepIdx: {
    fontWeight: 700,
    color: '#67e8f9',
    fontSize: '11.5px',
  },
  stepName: {
    fontWeight: 600,
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  dimText: {
    color: '#94a3b8',
    fontWeight: 400,
  },
  matchTag: {
    fontSize: '11.5px',
    fontWeight: 600,
    padding: '2px 8px',
    borderRadius: '99px',
    backgroundColor: 'rgba(16, 185, 129, 0.12)',
    color: '#6ee7b7',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(16, 185, 129, 0.35)',
    whiteSpace: 'nowrap',
  },
  vibeTag: {
    fontSize: '11.5px',
    fontWeight: 600,
    padding: '2px 8px',
    borderRadius: '99px',
    backgroundColor: 'rgba(6, 182, 212, 0.12)',
    color: '#67e8f9',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(6, 182, 212, 0.35)',
    whiteSpace: 'nowrap',
  },
  bpmTag: {
    fontSize: '12px',
    color: '#94a3b8',
    fontWeight: 600,
  },
  previewBtn: {
    height: '26px',
    padding: '0 10px',
    borderRadius: '4px',
    backgroundColor: '#161f30',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    color: '#f8fafc',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    whiteSpace: 'nowrap',
  },
  cueStrip: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#070a0f',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '5px',
    padding: '7px 12px',
  },
  cueStripTitle: {
    fontSize: '11.5px',
    color: '#94a3b8',
    fontWeight: 600,
  },
  cuePinsList: {
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
    fontSize: '11.5px',
    fontWeight: 500,
  },
  cuePin: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '5px',
  },
  dot: {
    width: '7px',
    height: '7px',
    borderRadius: '50%',
  },
  dotGreen: { backgroundColor: '#10b981' },
  dotBlue: { backgroundColor: '#3b82f6' },
  dotRed: { backgroundColor: '#f43f5e' },
  dotYellow: { backgroundColor: '#f59e0b' },
  badgeCyan: {
    fontSize: '11px',
    fontWeight: 700,
    padding: '2px 7px',
    borderRadius: '4px',
    backgroundColor: 'rgba(6, 182, 212, 0.14)',
    color: '#67e8f9',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(6, 182, 212, 0.35)',
  },
  badgeEmerald: {
    fontSize: '11px',
    fontWeight: 700,
    padding: '2px 7px',
    borderRadius: '4px',
    backgroundColor: 'rgba(16, 185, 129, 0.14)',
    color: '#6ee7b7',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(16, 185, 129, 0.35)',
  },
  tableWrap: {
    flex: 1,
    overflowY: 'auto',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    textAlign: 'left',
  },
  tableBtn: {
    height: '26px',
    padding: '0 10px',
    borderRadius: '4px',
    backgroundColor: 'rgba(6, 182, 212, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(6, 182, 212, 0.4)',
    color: '#67e8f9',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  copilotBody: {
    flex: 1,
    padding: '16px',
    display: 'flex',
    flexDirection: 'column',
    gap: '14px',
    overflowY: 'auto',
  },
  bossMsg: {
    alignSelf: 'flex-end',
    maxWidth: '90%',
    backgroundColor: '#161f30',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '8px 8px 2px 8px',
    padding: '10px 14px',
    color: '#f8fafc',
    fontSize: '13px',
  },
  bossHeader: {
    fontSize: '11px',
    fontWeight: 700,
    color: '#67e8f9',
    marginBottom: '4px',
    display: 'flex',
    justifyContent: 'space-between',
  },
  receiptBox: {
    backgroundColor: '#101622',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '8px',
    overflow: 'hidden',
    display: 'flex',
    flexDirection: 'column',
  },
  receiptHead: {
    padding: '10px 14px',
    backgroundColor: '#131b29',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  receiptTitle: {
    fontWeight: 700,
    fontSize: '12.5px',
    color: '#6ee7b7',
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
  },
  receiptBody: {
    padding: '12px 14px',
    display: 'flex',
    flexDirection: 'column',
    gap: '10px',
    fontSize: '12.5px',
    color: '#94a3b8',
  },
  metricsRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '5px',
    backgroundColor: '#090c12',
    overflow: 'hidden',
  },
  metricCell: {
    padding: '8px 10px',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#1f2a3d',
  },
  metricLabel: {
    fontSize: '10.5px',
    color: '#94a3b8',
  },
  metricVal: {
    fontSize: '12.5px',
    fontWeight: 700,
    marginTop: '2px',
  },
  humanInstructions: {
    display: 'flex',
    flexDirection: 'column',
    gap: '5px',
    fontSize: '12.5px',
    color: '#f8fafc',
    backgroundColor: '#0b0f17',
    padding: '10px 12px',
    borderRadius: '5px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
  },
  receiptActions: {
    padding: '10px 14px',
    backgroundColor: '#0c111a',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1f2a3d',
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
  },
  primaryBtn: {
    height: '32px',
    padding: '0 12px',
    borderRadius: '5px',
    backgroundColor: '#06b6d4',
    color: '#041318',
    fontWeight: 700,
    fontSize: '12px',
    borderWidth: 0,
    cursor: 'pointer',
    display: 'inline-flex',
    alignItems: 'center',
    gap: '5px',
    whiteSpace: 'nowrap',
  },
  secondaryBtn: {
    height: '32px',
    padding: '0 10px',
    borderRadius: '5px',
    backgroundColor: '#161f30',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    color: '#f8fafc',
    fontWeight: 600,
    fontSize: '12px',
    cursor: 'pointer',
    display: 'inline-flex',
    alignItems: 'center',
    gap: '5px',
    whiteSpace: 'nowrap',
  },
  statusAlert: {
    padding: '8px 12px',
    backgroundColor: 'rgba(6, 182, 212, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(6, 182, 212, 0.35)',
    borderRadius: '5px',
    fontSize: '12px',
    color: '#67e8f9',
    fontWeight: 600,
  },
  inputDock: {
    padding: '12px 16px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1f2a3d',
    backgroundColor: '#101622',
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
  },
  chipsList: {
    display: 'flex',
    gap: '6px',
    overflowX: 'auto',
  },
  chip: {
    fontSize: '11px',
    padding: '3px 8px',
    borderRadius: '99px',
    backgroundColor: '#161f30',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    color: '#f8fafc',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  },
  inputRow: {
    display: 'flex',
    gap: '8px',
  },
  chatInput: {
    flex: 1,
    height: '34px',
    backgroundColor: '#080b10',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '5px',
    padding: '0 12px',
    color: '#f8fafc',
    fontSize: '12.5px',
  },
  drawerOverlay: {
    position: 'fixed',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: 'rgba(5, 8, 13, 0.75)',
    backdropFilter: 'blur(3px)',
    zIndex: 999,
    display: 'flex',
    justifyContent: 'flex-end',
  },
  drawer: {
    width: '640px',
    height: '100%',
    backgroundColor: '#0d121b',
    borderLeftWidth: '1px',
    borderLeftStyle: 'solid',
    borderLeftColor: '#2e3d57',
    display: 'flex',
    flexDirection: 'column',
    boxShadow: '-16px 0 40px rgba(0, 0, 0, 0.65)',
  },
  drawerHeader: {
    padding: '16px 20px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#0a0e16',
  },
  closeBtn: {
    height: '28px',
    padding: '0 10px',
    borderRadius: '5px',
    backgroundColor: '#161f30',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    color: '#94a3b8',
    fontSize: '12px',
    cursor: 'pointer',
  },
  drawerBody: {
    flex: 1,
    padding: '16px 20px',
    display: 'flex',
    flexDirection: 'column',
    gap: '16px',
    overflowY: 'auto',
  },
  subHeadRow: {
    display: 'flex',
    justifyContent: 'space-between',
    marginBottom: '6px',
    fontSize: '12.5px',
  },
  authGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2e3d57',
    borderRadius: '6px',
    backgroundColor: '#080b10',
    overflow: 'hidden',
  },
  authCard: {
    padding: '10px 12px',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#1f2a3d',
    display: 'flex',
    flexDirection: 'column',
    gap: '3px',
    cursor: 'pointer',
  },
  authCardActive: {
    backgroundColor: 'rgba(6, 182, 212, 0.14)',
  },
  authTitle: {
    fontSize: '12.5px',
    fontWeight: 700,
    color: '#f8fafc',
    display: 'flex',
    justifyContent: 'space-between',
  },
  authDesc: {
    fontSize: '11px',
    color: '#94a3b8',
    lineHeight: '1.35',
  },
  modesContainer: {
    display: 'flex',
    flexDirection: 'column',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    borderRadius: '6px',
    backgroundColor: '#101622',
    overflow: 'hidden',
  },
  modeRow: {
    display: 'grid',
    gridTemplateColumns: '1fr 140px',
    alignItems: 'center',
    padding: '12px 14px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1f2a3d',
    gap: '12px',
    cursor: 'pointer',
  },
  modeRowActive: {
    backgroundColor: 'rgba(6, 182, 212, 0.08)',
  },
  modeTitle: {
    fontSize: '13px',
    fontWeight: 700,
    color: '#f8fafc',
  },
  modeDesc: {
    fontSize: '11.5px',
    color: '#94a3b8',
    marginTop: '2px',
  },
  badgesRow: {
    display: 'flex',
    gap: '6px',
    marginTop: '6px',
  },
  pillBadge: {
    fontSize: '11px',
    fontWeight: 600,
    padding: '2px 8px',
    borderRadius: '99px',
    backgroundColor: '#090c12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#1f2a3d',
    color: '#94a3b8',
  },
  pillActiveBadge: {
    fontSize: '11px',
    fontWeight: 600,
    padding: '2px 8px',
    borderRadius: '99px',
    backgroundColor: 'rgba(16, 185, 129, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(16, 185, 129, 0.4)',
    color: '#6ee7b7',
  },
  proExpanderRow: {
    padding: '10px 14px',
    backgroundColor: '#090c12',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  proBox: {
    padding: '12px 14px',
    backgroundColor: '#080b10',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1f2a3d',
    display: 'flex',
    flexDirection: 'column',
    gap: '8px',
  },
  swarmLockBar: {
    padding: '12px 14px',
    borderRadius: '6px',
    backgroundColor: 'rgba(16, 185, 129, 0.08)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(16, 185, 129, 0.35)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  toggleLocked: {
    fontSize: '11px',
    fontWeight: 700,
    padding: '3px 8px',
    borderRadius: '99px',
    backgroundColor: 'rgba(6, 182, 212, 0.14)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(6, 182, 212, 0.45)',
    color: '#67e8f9',
  },
});

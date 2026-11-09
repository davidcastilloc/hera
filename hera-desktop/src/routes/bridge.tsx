import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect, useState } from 'react';

import Icon from '../components/Icon';
import View from '../elements/View';
import {
  chatWithCloudflareFreeAi,
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
  getHumanMatchLabel,
  useHeraViewStore,
} from '../lib/hera-human-language';

export const Route = createFileRoute('/bridge')({
  component: ViewHarmonicBridgeP2P,
});

const PHRASE_RAIL_MARKERS = [
  {
    num: 1,
    simpleLabel: '1 · Entrada',
    proLabel: '1 · Intro',
    time: '00:16',
    pct: '5%',
    color: '#22c55e',
  },
  {
    num: 2,
    simpleLabel: '2 · Calma',
    proLabel: '2 · Breakdown',
    time: '01:48',
    pct: '33%',
    color: '#38bdf8',
  },
  {
    num: 3,
    simpleLabel: '3 · Explosión',
    proLabel: '3 · Drop',
    time: '02:34',
    pct: '48%',
    color: '#ef4444',
  },
  {
    num: 4,
    simpleLabel: '4 · Salida',
    proLabel: '4 · Outro',
    time: '05:12',
    pct: '92%',
    color: '#f59e0b',
  },
] as const;

function ViewHarmonicBridgeP2P() {
  const viewMode = useHeraViewStore((state) => state.viewMode);
  const isOnboardingOpen = useHeraViewStore((state) => state.isOnboardingOpen);
  const closeOnboarding = useHeraViewStore((state) => state.closeOnboarding);
  const openOnboarding = useHeraViewStore((state) => state.openOnboarding);

  const [originTitle] = useState('Levels (Original Mix)');
  const [originArtist] = useState('Avicii');
  const [originBpm] = useState(124.0);
  const [originCamelot] = useState('8A');

  const [targetTitle] = useState("Don't You Worry Child");
  const [targetArtist] = useState('Swedish House Mafia');
  const [targetBpm] = useState(128.0);
  const [targetCamelot] = useState('10A');

  const [bridgeResult, setBridgeResult] =
    useState<ResolveHarmonicBridgeResponse | null>(null);
  const [edgeRadar, setEdgeRadar] = useState<CloudflareEdgeCapabilities | null>(
    null,
  );
  const [previewStatus, setPreviewStatus] = useState<string>('');
  const [chatPrompt, setChatPrompt] = useState(
    "Quiero pasar de Levels a Don't You Worry Child subiendo el ritmo suave y sin que choquen las voces",
  );
  const [selectedBackend, setSelectedBackend] = useState<string>('cli_agy');
  const [customBaseUrl, setCustomBaseUrl] = useState<string>(
    'http://localhost:1234/v1',
  );
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

  const handleRunOnboardingScan = useCallback(async () => {
    setIsScanningOnboarding(true);
    try {
      const rep = await scanAiConnectivityOnboarding(commanderMode);
      setOnboardingReport(rep);
      if (rep.recommended_backend_id) {
        setSelectedBackend(rep.recommended_backend_id);
      }
      setProbeBanner(
        viewMode === 'simple'
          ? `Motor listo: ${rep.recommended_backend_id.replace('cli_', '')}`
          : `${rep.summary_badge} · Quarantine-First`,
      );
    } catch {
      // Offline fallback
    } finally {
      setIsScanningOnboarding(false);
    }
  }, [commanderMode, viewMode]);

  const handleSendCopilotCommand = async (customText?: string) => {
    const promptToSend = customText || chatPrompt;
    if (!promptToSend.trim()) return;

    setIsAiLoading(true);
    try {
      const reply = await chatWithCloudflareFreeAi(promptToSend, 'DaviDJ', {
        backend: selectedBackend,
        baseUrl: customBaseUrl,
        commanderMode,
      });
      setAiResponse(reply);
      setPreviewStatus(
        viewMode === 'simple'
          ? 'Propuesta actualizada · Revisa los pasos antes de agregar al crate'
          : `Respuesta generada por ${reply.model_used}`,
      );
    } catch {
      setPreviewStatus('Conectando con el motor del copiloto...');
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
        // Offline fallback
      }

      try {
        const res = await resolveHarmonicBridge({
          originTitle,
          originArtist,
          originBpm,
          originCamelot,
          targetTitle,
          targetArtist,
          targetBpm,
          targetCamelot,
        });
        setBridgeResult(res);
      } catch {
        // Offline fallback
      }

      void handleRunOnboardingScan();
    };

    void loadInitialData();
  }, [
    handleRunOnboardingScan,
    originArtist,
    originBpm,
    originCamelot,
    originTitle,
    targetArtist,
    targetBpm,
    targetCamelot,
    targetTitle,
  ]);

  const matchInfo = getHumanMatchLabel(
    originCamelot,
    targetCamelot,
    originBpm,
    targetBpm,
  );

  return (
    <View aria-label="Puente Armónico y Copiloto de Cabina">
      <div {...stylex.props(styles.pageContainer)}>
        <header {...stylex.props(styles.topBar)}>
          <div {...stylex.props(styles.brandGroup)}>
            <Icon name="bridge" size={18} color="#2dd4bf" />
            <h1 {...stylex.props(styles.pageTitle)}>
              {viewMode === 'simple'
                ? 'Puente Musical y Copiloto de Cabina'
                : 'Puente Armónico 2-Opt y Enjambre P2P'}
            </h1>
          </div>

          <div {...stylex.props(styles.topActions)}>
            <button
              type="button"
              onClick={openOnboarding}
              {...stylex.props(styles.statusPill, styles.activePill)}
              title="Configurar motor de IA y autoridad de cabina"
            >
              <span {...stylex.props(styles.emeraldDot)} />
              <span>
                {viewMode === 'simple'
                  ? `Copiloto: ${selectedBackend.replace('cli_', '')} · Control manual`
                  : `Motor: ${selectedBackend} · ${commanderMode}`}
              </span>
            </button>

            <button
              type="button"
              onClick={openOnboarding}
              {...stylex.props(styles.statusPill)}
            >
              <Icon name="cpu" size={14} />
              <span>{viewMode === 'simple' ? 'Motor IA' : '5 Modos IA'}</span>
            </button>

            <div {...stylex.props(styles.swarmPill)}>
              <Icon name="checkCircle" size={14} color="#2dd4bf" />
              <span>
                {viewMode === 'simple'
                  ? 'Enjambre conectado'
                  : `${edgeRadar?.active_swarm_nodes ?? 3} nodos · ${bridgeResult?.bridge_steps.length ?? 2} pasos`}
              </span>
            </div>
          </div>
        </header>

        <div {...stylex.props(styles.splitWorkspace)}>
          <section {...stylex.props(styles.leftCol)}>
            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Secuencia sugerida entre ambas pistas'
                  : 'Ruta óptima Deck A a Deck B (2-Opt TSP)'}
              </h2>
              <span {...stylex.props(styles.matchTag)}>
                {viewMode === 'simple'
                  ? `${matchInfo.compatibilityLabel} · ${matchInfo.tempoAdvice}`
                  : `Delta +${(targetBpm - originBpm).toFixed(1)} BPM · ${originCamelot} a ${targetCamelot}`}
              </span>
            </div>

            <div {...stylex.props(styles.deckRack)}>
              <div {...stylex.props(styles.decksRow)}>
                <div {...stylex.props(styles.deckCard)}>
                  <div {...stylex.props(styles.deckTopLine)}>
                    <span {...stylex.props(styles.deckRole)}>
                      {viewMode === 'simple'
                        ? 'Deck A · En reproducción'
                        : 'Deck A · Origen'}
                    </span>
                    <span {...stylex.props(styles.deckMetricBadge)}>
                      {originCamelot} · {originBpm.toFixed(1)} BPM
                    </span>
                  </div>
                  <div {...stylex.props(styles.deckSongTitle)}>
                    {originArtist} — {originTitle}
                  </div>
                  <div {...stylex.props(styles.deckDetails)}>
                    <span>Sonido de estudio</span>
                    <span>·</span>
                    <span>
                      Salida limpia: <strong>04:42</strong>
                    </span>
                  </div>
                </div>

                <div {...stylex.props(styles.arrowCol)} aria-hidden="true">
                  →
                </div>

                <div {...stylex.props(styles.deckCard)}>
                  <div {...stylex.props(styles.deckTopLine)}>
                    <span {...stylex.props(styles.deckRole)}>
                      {viewMode === 'simple'
                        ? 'Deck B · Pista destino'
                        : 'Deck B · Destino'}
                    </span>
                    <span {...stylex.props(styles.deckMetricBadge)}>
                      {targetCamelot} · {targetBpm.toFixed(1)} BPM
                    </span>
                  </div>
                  <div {...stylex.props(styles.deckSongTitle)}>
                    {targetArtist} — {targetTitle}
                  </div>
                  <div {...stylex.props(styles.deckDetails)}>
                    <span>Sonido de estudio</span>
                    <span>·</span>
                    <span>
                      Entrada limpia: <strong>00:15</strong>
                    </span>
                  </div>
                </div>
              </div>

              <div {...stylex.props(styles.stepsContainer)}>
                <div {...stylex.props(styles.stepRow)}>
                  <span {...stylex.props(styles.stepIdx)}>Paso 1</span>
                  <div {...stylex.props(styles.stepName)}>
                    Eric Prydz — Opus{' '}
                    <span {...stylex.props(styles.dimText)}>
                      {viewMode === 'simple'
                        ? '· Subida progresiva'
                        : '· 8A a 9A'}
                    </span>
                  </div>
                  <span {...stylex.props(styles.matchTag)}>Afinidad 98%</span>
                  <span {...stylex.props(styles.bpmTag)}>125.5 BPM</span>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus(
                        'Preescuchando transición de 15s (Cue 4 Salida a Cue 1 Entrada)...',
                      )
                    }
                    {...stylex.props(styles.previewBtn)}
                  >
                    <Icon name="play" size={12} />
                    <span>
                      {viewMode === 'simple' ? 'Oír cruce 15s' : 'Cruce 4→1'}
                    </span>
                  </button>
                </div>

                <div {...stylex.props(styles.stepRow)}>
                  <span {...stylex.props(styles.stepIdx)}>Paso 2</span>
                  <div {...stylex.props(styles.stepName)}>
                    Deadmau5 — Strobe{' '}
                    <span {...stylex.props(styles.dimText)}>
                      {viewMode === 'simple'
                        ? '· Prepara entrada vocal'
                        : '· 9A a 10A'}
                    </span>
                  </div>
                  <span {...stylex.props(styles.matchTag)}>Afinidad 97%</span>
                  <span {...stylex.props(styles.bpmTag)}>127.0 BPM</span>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus(
                        'Preescuchando transición de 15s (Cue 4 Salida a Cue 1 Entrada)...',
                      )
                    }
                    {...stylex.props(styles.previewBtn)}
                  >
                    <Icon name="play" size={12} />
                    <span>
                      {viewMode === 'simple' ? 'Oír cruce 15s' : 'Cruce 4→1'}
                    </span>
                  </button>
                </div>
              </div>

              <div {...stylex.props(styles.phraseRailPanel)}>
                <div {...stylex.props(styles.phraseRailHeader)}>
                  <span {...stylex.props(styles.phraseRailTitle)}>
                    {viewMode === 'simple'
                      ? 'Mapa proporcional de 4 Hot Cues de frase'
                      : 'Riel acústico de 4 Hot Cues (detección espectral)'}
                  </span>
                  <span {...stylex.props(styles.dimText)}>Duración 05:30</span>
                </div>

                <div {...stylex.props(styles.phraseTimelineTrack)}>
                  {PHRASE_RAIL_MARKERS.map((marker) => (
                    <span
                      key={marker.num}
                      {...stylex.props(styles.phraseTimelineTick)}
                      style={{
                        left: marker.pct,
                        backgroundColor: marker.color,
                      }}
                      title={`${marker.simpleLabel} (${marker.time})`}
                    />
                  ))}
                </div>

                <div {...stylex.props(styles.cuePinsList)}>
                  {PHRASE_RAIL_MARKERS.map((marker) => (
                    <span key={marker.num} {...stylex.props(styles.cuePin)}>
                      <span
                        {...stylex.props(styles.cueDot)}
                        style={{ backgroundColor: marker.color }}
                      />
                      <span>
                        {viewMode === 'simple'
                          ? marker.simpleLabel
                          : marker.proLabel}
                      </span>
                      <span {...stylex.props(styles.cueTime)}>
                        {marker.time}
                      </span>
                    </span>
                  ))}
                </div>
              </div>
            </div>

            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Candidatos compatibles en la red de cabinas'
                  : 'Índice federado del enjambre P2P'}
              </h2>
              <span {...stylex.props(styles.dimText)}>
                {viewMode === 'simple'
                  ? 'Calidad de estudio verificada'
                  : 'Auditoría espectral FFT > 20 kHz'}
              </span>
            </div>

            <div {...stylex.props(styles.tableWrap)}>
              <table {...stylex.props(styles.table)}>
                <thead>
                  <tr>
                    <th {...stylex.props(styles.th)}>Pista / Artista</th>
                    <th {...stylex.props(styles.th)}>Compatibilidad</th>
                    <th {...stylex.props(styles.th)}>BPM</th>
                    <th {...stylex.props(styles.th)}>Calidad</th>
                    <th {...stylex.props(styles.th)}>Acción</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td {...stylex.props(styles.td)}>
                      <strong>Pryda — Allein</strong>{' '}
                      <span {...stylex.props(styles.dimText)}>
                        · Original Mix
                      </span>
                    </td>
                    <td {...stylex.props(styles.td)}>
                      <span {...stylex.props(styles.matchTag)}>
                        Instrumental · 9A
                      </span>
                    </td>
                    <td {...stylex.props(styles.td, styles.tabularCell)}>
                      126.0
                    </td>
                    <td {...stylex.props(styles.td, styles.verifiedCell)}>
                      Estudio (Lossless)
                    </td>
                    <td {...stylex.props(styles.td)}>
                      <button
                        type="button"
                        onClick={() =>
                          setPreviewStatus(
                            'Añadido «Pryda — Allein» al crate de prueba',
                          )
                        }
                        {...stylex.props(styles.tableBtn)}
                      >
                        <Icon name="plus" size={12} />
                        <span>Probar</span>
                      </button>
                    </td>
                  </tr>
                  <tr>
                    <td {...stylex.props(styles.td)}>
                      <strong>RÜFÜS DU SOL — Innerbloom</strong>{' '}
                      <span {...stylex.props(styles.dimText)}>
                        · Lane 8 Remix
                      </span>
                    </td>
                    <td {...stylex.props(styles.td)}>
                      <span {...stylex.props(styles.matchTag)}>
                        Melódica · 8A
                      </span>
                    </td>
                    <td {...stylex.props(styles.td, styles.tabularCell)}>
                      124.5
                    </td>
                    <td {...stylex.props(styles.td, styles.verifiedCell)}>
                      Estudio (Lossless)
                    </td>
                    <td {...stylex.props(styles.td)}>
                      <button
                        type="button"
                        onClick={() =>
                          setPreviewStatus(
                            'Añadido «Innerbloom (Lane 8 Remix)» al crate de prueba',
                          )
                        }
                        {...stylex.props(styles.tableBtn)}
                      >
                        <Icon name="plus" size={12} />
                        <span>Probar</span>
                      </button>
                    </td>
                  </tr>
                  <tr>
                    <td {...stylex.props(styles.td)}>
                      <strong>ARTBAT — Flame</strong>{' '}
                      <span {...stylex.props(styles.dimText)}>
                        · Extended Mix
                      </span>
                    </td>
                    <td {...stylex.props(styles.td)}>
                      <span {...stylex.props(styles.vibeTag)}>
                        Sube energía · 9A
                      </span>
                    </td>
                    <td {...stylex.props(styles.td, styles.tabularCell)}>
                      126.0
                    </td>
                    <td {...stylex.props(styles.td, styles.verifiedCell)}>
                      Estudio (Lossless)
                    </td>
                    <td {...stylex.props(styles.td)}>
                      <button
                        type="button"
                        onClick={() =>
                          setPreviewStatus(
                            'Añadido «ARTBAT — Flame» al crate de prueba',
                          )
                        }
                        {...stylex.props(styles.tableBtn)}
                      >
                        <Icon name="plus" size={12} />
                        <span>Probar</span>
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          <section {...stylex.props(styles.rightCol)}>
            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Copiloto de Cabina'
                  : 'Consola ReAct de Curaduría'}
              </h2>
              <span {...stylex.props(styles.freeMeta)}>
                {viewMode === 'simple'
                  ? 'Aprobación manual por cambio'
                  : `${selectedBackend} · $0.00`}
              </span>
            </div>

            <div {...stylex.props(styles.copilotBody)}>
              <div {...stylex.props(styles.bossMsg)}>
                <div {...stylex.props(styles.bossHeader)}>
                  <strong>Tu instrucción</strong>
                  <span>Activa</span>
                </div>
                Quiero pasar de «Levels» a «Don&apos;t You Worry Child» subiendo
                el ritmo de forma progresiva y sin superponer vocales.
              </div>

              <div {...stylex.props(styles.receiptBox)}>
                <div {...stylex.props(styles.receiptHead)}>
                  <div {...stylex.props(styles.receiptTitle)}>
                    <span {...stylex.props(styles.emeraldDot)} />
                    <span>
                      {viewMode === 'simple'
                        ? 'Puente de 2 pistas listo para cabina'
                        : 'Secuencia 2-Opt TSP verificada'}
                    </span>
                  </div>
                  <span {...stylex.props(styles.dimText)}>
                    {viewMode === 'simple'
                      ? 'Audio verificado'
                      : 'Sinergia 97%'}
                  </span>
                </div>

                <div {...stylex.props(styles.receiptBody)}>
                  <p {...stylex.props(styles.receiptParagraph)}>
                    {aiResponse
                      ? aiResponse.assistant_reply
                      : viewMode === 'simple'
                        ? 'Para evitar un salto brusco de 124 a 128 BPM, intercalé dos cortes instrumentales que elevan la tensión de forma natural:'
                        : 'Curva calculada con incremento medio de +1.5 BPM por transición y continuidad armónica 8A → 9A → 10A:'}
                  </p>

                  <div {...stylex.props(styles.metricsRow)}>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricVal)}>+1.5 BPM</div>
                      <div {...stylex.props(styles.metricName)}>
                        Pendiente por paso
                      </div>
                    </div>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricVal)}>Cue 4 → 1</div>
                      <div {...stylex.props(styles.metricName)}>
                        Cero choque vocal
                      </div>
                    </div>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricVal)}>Lossless</div>
                      <div {...stylex.props(styles.metricName)}>
                        Espectro completo
                      </div>
                    </div>
                  </div>

                  <div {...stylex.props(styles.humanInstructions)}>
                    <div>
                      <strong>1. Eric Prydz — Opus:</strong> mezcla sobre la
                      salida instrumental de Levels en 04:42.
                    </div>
                    <div>
                      <strong>2. Deadmau5 — Strobe:</strong> entra limpio en
                      05:12 manteniendo el pulso grave.
                    </div>
                    <div>
                      <strong>3. Don&apos;t You Worry Child:</strong> lanza en
                      128.0 BPM con entrada armónica en 10A.
                    </div>
                  </div>
                </div>

                <div {...stylex.props(styles.receiptActions)}>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus(
                        'Puente aprobado e incorporado a tu crate activo',
                      )
                    }
                    {...stylex.props(styles.primaryBtn)}
                  >
                    <Icon name="checkCircle" size={14} />
                    <span>
                      {viewMode === 'simple'
                        ? 'Agregar al Set'
                        : 'Aprobar y Promover'}
                    </span>
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus(
                        'Exportando puente con 4 Hot Cues a VirtualDJ...',
                      )
                    }
                    {...stylex.props(styles.secondaryBtn)}
                  >
                    <Icon name="export" size={14} />
                    <span>Exportar a VirtualDJ</span>
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus(
                        'Reproduciendo simulación completa del puente...',
                      )
                    }
                    {...stylex.props(styles.secondaryBtn)}
                  >
                    <Icon name="play" size={12} />
                    <span>Escuchar puente</span>
                  </button>
                </div>
              </div>

              {previewStatus && (
                <div {...stylex.props(styles.statusAlert)}>
                  <Icon name="checkCircle" size={14} color="#2dd4bf" />
                  <span>{previewStatus}</span>
                </div>
              )}
            </div>

            <div {...stylex.props(styles.inputDock)}>
              <div {...stylex.props(styles.chipsList)}>
                <button
                  type="button"
                  onClick={() =>
                    handleSendCopilotCommand(
                      'Propón una alternativa con más energía para hora pico',
                    )
                  }
                  {...stylex.props(styles.chip)}
                >
                  Subir energía
                </button>
                <button
                  type="button"
                  onClick={() =>
                    handleSendCopilotCommand(
                      'Reduce la pendiente de BPM para una transición más larga',
                    )
                  }
                  {...stylex.props(styles.chip)}
                >
                  Transición más suave
                </button>
                <button
                  type="button"
                  onClick={() =>
                    handleSendCopilotCommand(
                      'Prioriza pistas 100% instrumentales en la entrada',
                    )
                  }
                  {...stylex.props(styles.chip)}
                >
                  Solo entradas sin voz
                </button>
              </div>

              <div {...stylex.props(styles.inputRow)}>
                <input
                  type="text"
                  value={chatPrompt}
                  onChange={(e) => setChatPrompt(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void handleSendCopilotCommand();
                  }}
                  placeholder="Indica qué ajuste quieres en el puente o en tu set..."
                  {...stylex.props(styles.chatInput)}
                />
                <button
                  type="button"
                  onClick={() => handleSendCopilotCommand()}
                  disabled={isAiLoading}
                  {...stylex.props(styles.primaryBtn)}
                >
                  {isAiLoading ? 'Calculando...' : 'Enviar'}
                </button>
              </div>
            </div>
          </section>
        </div>

        {isOnboardingOpen && (
          <div
            {...stylex.props(styles.drawerOverlay)}
            role="dialog"
            aria-modal="true"
            aria-label="Configuración de Motor IA y Autoridad de Cabina"
          >
            <button
              type="button"
              aria-label="Cerrar panel de configuración"
              onClick={closeOnboarding}
              {...stylex.props(styles.drawerBackdropBtn)}
            />
            <aside {...stylex.props(styles.drawer)}>
              <div {...stylex.props(styles.drawerHeader)}>
                <div>
                  <h2 {...stylex.props(styles.drawerTitle)}>
                    Configuración de Motor IA y Autoridad de Cabina
                  </h2>
                  <p {...stylex.props(styles.drawerSub)}>
                    Selecciona cómo interviene el copiloto y qué motor local o
                    en la nube procesa tus consultas.
                  </p>
                </div>
                <button
                  type="button"
                  onClick={closeOnboarding}
                  {...stylex.props(styles.closeBtn)}
                >
                  Cerrar
                </button>
              </div>

              <div {...stylex.props(styles.drawerBody)}>
                <div {...stylex.props(styles.drawerSection)}>
                  <div {...stylex.props(styles.subHeadRow)}>
                    <strong>1. Nivel de autonomía en cabina</strong>
                    <span {...stylex.props(styles.freeMeta)}>
                      {commanderMode === 'executive_boss'
                        ? 'Aprobación manual'
                        : commanderMode === 'autonomous_operator'
                          ? 'Piloto automático'
                          : 'Solo lectura'}
                    </span>
                  </div>

                  <div {...stylex.props(styles.authGrid)}>
                    <button
                      type="button"
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'executive_boss' &&
                          styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('executive_boss')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        Control manual del DJ
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        El copiloto sugiere puentes y ordenamientos; tú apruebas
                        con un clic antes de aplicar cambios.
                      </div>
                    </button>

                    <button
                      type="button"
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'autonomous_operator' &&
                          styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('autonomous_operator')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        Curaduría automática
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        Ordena crates y prepara exportaciones automáticamente
                        respetando tus invariantes acústicos.
                      </div>
                    </button>

                    <button
                      type="button"
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'advisor_only' &&
                          styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('advisor_only')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        Modo consulta
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        Muestra sugerencias de compatibilidad sin modificar
                        listas ni archivos.
                      </div>
                    </button>
                  </div>
                </div>

                <div {...stylex.props(styles.drawerSection)}>
                  <div {...stylex.props(styles.subHeadRow)}>
                    <strong>2. Motor de inferencia ($0.00 incluidos)</strong>
                    <span {...stylex.props(styles.freeMeta)}>
                      {isScanningOnboarding
                        ? 'Detectando motores...'
                        : probeBanner ||
                          (onboardingReport
                            ? `${onboardingReport.cli_agents.length} motores locales detectados`
                            : 'Listo')}
                    </span>
                  </div>

                  <div {...stylex.props(styles.modesContainer)}>
                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend.startsWith('cli_') &&
                          styles.modeRowActive,
                      )}
                    >
                      <div {...stylex.props(styles.modeInfo)}>
                        <div {...stylex.props(styles.modeTitle)}>
                          Agentes CLI locales en este equipo
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          Aprovecha tus herramientas locales instaladas sin
                          configurar llaves adicionales.
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
                        {selectedBackend === 'cli_agy'
                          ? 'Activo'
                          : 'Seleccionar'}
                      </button>
                    </div>

                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend === 'workers_ai_llama33' &&
                          styles.modeRowActive,
                      )}
                    >
                      <div {...stylex.props(styles.modeInfo)}>
                        <div {...stylex.props(styles.modeTitle)}>
                          Cloudflare Workers AI Free (10k Neurons/día)
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          Motor en la nube incluido de forma gratuita sin
                          instalar modelos pesados.
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
                        {selectedBackend === 'workers_ai_llama33'
                          ? 'Activo'
                          : 'Seleccionar'}
                      </button>
                    </div>

                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend === 'ollama' && styles.modeRowActive,
                      )}
                    >
                      <div {...stylex.props(styles.modeInfo)}>
                        <div {...stylex.props(styles.modeTitle)}>
                          Cabina Offline (Ollama / LM Studio local)
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          100% sin conexión a internet para sets en vivo en
                          clubes o eventos remotos.
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
                        {selectedBackend === 'ollama'
                          ? 'Activo'
                          : 'Seleccionar'}
                      </button>
                    </div>

                    <div {...stylex.props(styles.proExpanderRow)}>
                      <span {...stylex.props(styles.dimText)}>
                        Servidor OpenAI-compatible personalizado o protocolo MCP
                      </span>
                      <button
                        type="button"
                        onClick={() => setShowProOptions(!showProOptions)}
                        {...stylex.props(styles.secondaryBtn)}
                      >
                        {showProOptions ? 'Ocultar' : 'Configurar'}
                      </button>
                    </div>

                    {showProOptions && (
                      <div {...stylex.props(styles.proBox)}>
                        <div {...stylex.props(styles.modeDesc)}>
                          Servidor MCP nativo disponible mediante{' '}
                          <code>hera-node --mcp</code>.
                        </div>
                        <div {...stylex.props(styles.inputRow)}>
                          <input
                            type="text"
                            value={customBaseUrl}
                            onChange={(e) => setCustomBaseUrl(e.target.value)}
                            placeholder="http://localhost:1234/v1"
                            {...stylex.props(styles.chatInput)}
                          />
                          <button
                            type="button"
                            onClick={() => {
                              void probeLocalAiBackend(customBaseUrl).then(
                                (probe) => {
                                  setPreviewStatus(
                                    probe.reachable
                                      ? `Servidor local verificado (${probe.base_url})`
                                      : `URL local guardada (${customBaseUrl})`,
                                  );
                                },
                              );
                            }}
                            {...stylex.props(styles.secondaryBtn)}
                          >
                            Verificar
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                <div {...stylex.props(styles.swarmLockBar)}>
                  <div {...stylex.props(styles.modeInfo)}>
                    <div {...stylex.props(styles.modeTitle)}>
                      Invariante colaborativo del enjambre HERA
                    </div>
                    <div {...stylex.props(styles.modeDesc)}>
                      El motor DSP, los 4 Hot Cues y los exportadores DJ son
                      100% libres manteniendo activa la red P2P comunitaria.
                    </div>
                  </div>
                  <span {...stylex.props(styles.swarmActiveTag)}>
                    Siempre activo
                  </span>
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
    backgroundColor: '#0c0e12',
    color: '#edeff3',
    overflow: 'hidden',
  },
  topBar: {
    height: '46px',
    flexShrink: 0,
    backgroundColor: '#12161d',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    paddingBlock: 0,
    paddingInline: '18px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  brandGroup: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    minWidth: 0,
  },
  pageTitle: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
  },
  topActions: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    flexShrink: 0,
  },
  statusPill: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    fontSize: '12px',
    fontWeight: 500,
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    cursor: 'pointer',
  },
  activePill: {
    borderColor: 'rgba(20, 184, 166, 0.32)',
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
  },
  swarmPill: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    fontSize: '12px',
    fontWeight: 500,
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
  },
  emeraldDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  splitWorkspace: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    display: 'grid',
    gridTemplateColumns: '1.25fr 0.95fr',
    overflow: 'hidden',
  },
  leftCol: {
    padding: '18px',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '14px',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#232a36',
    minWidth: 0,
  },
  rightCol: {
    padding: '18px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
    backgroundColor: '#10141b',
    minWidth: 0,
    minHeight: 0,
    overflow: 'hidden',
  },
  sectionHead: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '10px',
    flexWrap: 'wrap',
    rowGap: '4px',
  },
  sectionTitle: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  matchTag: {
    display: 'inline-flex',
    alignItems: 'center',
    paddingBlock: '2px',
    paddingInline: '8px',
    borderRadius: '4px',
    fontSize: '11.5px',
    fontWeight: 600,
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
  },
  vibeTag: {
    display: 'inline-flex',
    alignItems: 'center',
    paddingBlock: '2px',
    paddingInline: '8px',
    borderRadius: '4px',
    fontSize: '11.5px',
    fontWeight: 600,
    backgroundColor: 'rgba(245, 158, 11, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(245, 158, 11, 0.28)',
    color: '#fbbf24',
  },
  deckRack: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  decksRow: {
    display: 'grid',
    gridTemplateColumns: '1fr auto 1fr',
    alignItems: 'center',
    columnGap: '10px',
  },
  deckCard: {
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    padding: '12px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    minWidth: 0,
  },
  deckTopLine: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
  },
  deckRole: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#8e98a8',
  },
  deckMetricBadge: {
    fontSize: '11px',
    fontWeight: 600,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    paddingBlock: '2px',
    paddingInline: '7px',
    borderRadius: '4px',
    fontVariantNumeric: 'tabular-nums',
  },
  deckSongTitle: {
    fontSize: '13.5px',
    fontWeight: 700,
    color: '#edeff3',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  deckDetails: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    fontSize: '11.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  arrowCol: {
    color: '#8e98a8',
    fontSize: '15px',
    fontWeight: 600,
  },
  stepsContainer: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
  },
  stepRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    paddingBlock: '8px',
    paddingInline: '10px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
  },
  stepIdx: {
    fontSize: '11px',
    fontWeight: 600,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    paddingBlock: '2px',
    paddingInline: '7px',
    borderRadius: '4px',
    flexShrink: 0,
  },
  stepName: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    fontSize: '12.5px',
    fontWeight: 600,
    color: '#edeff3',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  dimText: {
    color: '#8e98a8',
    fontSize: '11.5px',
    fontWeight: 400,
  },
  bpmTag: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  previewBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '4px',
    paddingInline: '9px',
    borderRadius: '5px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
    flexShrink: 0,
  },
  phraseRailPanel: {
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    paddingBlock: '10px',
    paddingInline: '12px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  phraseRailHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  phraseRailTitle: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  phraseTimelineTrack: {
    position: 'relative',
    height: '6px',
    width: '100%',
    backgroundColor: '#171c24',
    borderRadius: '999px',
  },
  phraseTimelineTick: {
    position: 'absolute',
    top: '-3px',
    width: '6px',
    height: '12px',
    borderRadius: '2px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#0c0e12',
    transform: 'translateX(-3px)',
  },
  cuePinsList: {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    justifyContent: 'space-between',
    rowGap: '6px',
    columnGap: '10px',
  },
  cuePin: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    fontSize: '11.5px',
    color: '#edeff3',
  },
  cueDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    flexShrink: 0,
  },
  cueTime: {
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  tableWrap: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    overflow: 'hidden',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '12.5px',
  },
  th: {
    textAlign: 'left',
    paddingBlock: '9px',
    paddingInline: '12px',
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#8e98a8',
    backgroundColor: '#171c24',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
  },
  td: {
    paddingBlock: '9px',
    paddingInline: '12px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: 'rgba(35, 42, 54, 0.6)',
    color: '#edeff3',
  },
  tabularCell: {
    fontVariantNumeric: 'tabular-nums',
    fontWeight: 600,
  },
  verifiedCell: {
    color: '#2dd4bf',
    fontSize: '12px',
    fontWeight: 500,
  },
  tableBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '4px',
    paddingInline: '9px',
    borderRadius: '5px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  freeMeta: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#2dd4bf',
  },
  copilotBody: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  bossMsg: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '12px',
    fontSize: '12.5px',
    lineHeight: 1.5,
    color: '#edeff3',
  },
  bossHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '4px',
    fontSize: '11.5px',
    color: '#8e98a8',
  },
  receiptBox: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  receiptHead: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
  },
  receiptTitle: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    fontSize: '13px',
    fontWeight: 700,
    color: '#edeff3',
  },
  receiptBody: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  receiptParagraph: {
    margin: 0,
    fontSize: '12.5px',
    lineHeight: 1.5,
    color: '#8e98a8',
  },
  metricsRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '8px',
  },
  metricCell: {
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    padding: '9px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
  },
  metricVal: {
    fontSize: '13px',
    fontWeight: 700,
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
  },
  metricName: {
    fontSize: '11px',
    color: '#8e98a8',
  },
  humanInstructions: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    fontSize: '12px',
    lineHeight: 1.45,
    color: '#edeff3',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    padding: '10px',
    fontVariantNumeric: 'tabular-nums',
  },
  receiptActions: {
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '8px',
    columnGap: '8px',
  },
  primaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '13px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontSize: '12px',
    fontWeight: 700,
    cursor: 'pointer',
    flexShrink: 0,
  },
  secondaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '11px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: '#171c24',
    color: '#edeff3',
    fontSize: '12px',
    fontWeight: 600,
    cursor: 'pointer',
    flexShrink: 0,
  },
  statusAlert: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '8px',
    paddingInline: '12px',
    borderRadius: '6px',
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    color: '#2dd4bf',
    fontSize: '12px',
    fontWeight: 500,
  },
  inputDock: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
    paddingTop: '10px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#232a36',
    flexShrink: 0,
  },
  chipsList: {
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '6px',
    columnGap: '6px',
  },
  chip: {
    paddingBlock: '4px',
    paddingInline: '9px',
    borderRadius: '999px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  inputRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  chatInput: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    paddingBlock: '8px',
    paddingInline: '11px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12.5px',
  },
  drawerOverlay: {
    position: 'fixed',
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
    backgroundColor: 'rgba(6, 8, 12, 0.72)',
    display: 'flex',
    justifyContent: 'flex-end',
    zIndex: 1000,
  },
  drawerBackdropBtn: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    borderWidth: 0,
    padding: 0,
    margin: 0,
    backgroundColor: 'transparent',
    cursor: 'default',
  },
  drawer: {
    width: '520px',
    maxWidth: '92vw',
    height: '100%',
    backgroundColor: '#12161d',
    borderLeftWidth: '1px',
    borderLeftStyle: 'solid',
    borderLeftColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  drawerHeader: {
    paddingBlock: '16px',
    paddingInline: '20px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  drawerTitle: {
    margin: 0,
    fontSize: '15px',
    fontWeight: 700,
    color: '#edeff3',
  },
  drawerSub: {
    marginTop: '4px',
    marginRight: 0,
    marginBottom: 0,
    marginLeft: 0,
    fontSize: '12px',
    color: '#8e98a8',
    lineHeight: 1.4,
  },
  closeBtn: {
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12px',
    cursor: 'pointer',
  },
  drawerBody: {
    padding: '20px',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '18px',
  },
  drawerSection: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
  },
  subHeadRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    fontSize: '13px',
    color: '#edeff3',
  },
  authGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '8px',
  },
  authCard: {
    textAlign: 'left',
    padding: '11px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    cursor: 'pointer',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '5px',
  },
  authCardActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderColor: 'rgba(20, 184, 166, 0.4)',
  },
  authTitle: {
    fontSize: '12px',
    fontWeight: 700,
    color: '#edeff3',
  },
  authDesc: {
    fontSize: '11.5px',
    lineHeight: 1.4,
    color: '#8e98a8',
  },
  modesContainer: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  modeRow: {
    padding: '12px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  modeRowActive: {
    borderColor: 'rgba(20, 184, 166, 0.36)',
  },
  modeInfo: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
    minWidth: 0,
  },
  modeTitle: {
    fontSize: '12.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  modeDesc: {
    fontSize: '11.5px',
    lineHeight: 1.4,
    color: '#8e98a8',
  },
  proExpanderRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '10px',
    paddingTop: '4px',
  },
  proBox: {
    padding: '12px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  swarmLockBar: {
    padding: '12px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  swarmActiveTag: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#2dd4bf',
    flexShrink: 0,
  },
});

import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

import View from '../elements/View';
import {
  chatWithCloudflareFreeAi,
  probeLocalAiBackend,
  resolveHarmonicBridge,
  type HeraCloudflareFreeAiResponse,
  type ResolveHarmonicBridgeResponse,
} from '../lib/bridge-hera';
import Player from '../lib/player';

export const Route = createFileRoute('/bridge')({
  component: ViewHarmonicBridgeP2P,
});

function ViewHarmonicBridgeP2P() {
  const [originTitle, setOriginTitle] = useState('Baiana (Club Mix)');
  const [originArtist, setOriginArtist] = useState('Barbatuques');
  const [originBpm, setOriginBpm] = useState(118.0);
  const [originCamelot, setOriginCamelot] = useState('8A');

  const [targetTitle, setTargetTitle] = useState('Pepas (Guaracha VIP)');
  const [targetArtist, setTargetArtist] = useState('Farruko');
  const [targetBpm, setTargetBpm] = useState(130.0);
  const [targetCamelot, setTargetCamelot] = useState('10A');

  const [bridgeResult, setBridgeResult] =
    useState<ResolveHarmonicBridgeResponse | null>(null);
  const [previewStatus, setPreviewStatus] = useState<string>('');
  const [turboCreditsMb, setTurboCreditsMb] = useState<number>(150.0);
  const [chatPrompt, setChatPrompt] = useState(
    'Armar transición de 118 BPM (8A) a 130 BPM (10A) y exportar a VirtualDJ',
  );
  const [selectedBackend, setSelectedBackend] = useState<string>('lmstudio');
  const [customBaseUrl, setCustomBaseUrl] = useState<string>(
    'http://localhost:1234/v1',
  );
  const [customModel, setCustomModel] = useState<string>('default');
  const [probeBanner, setProbeBanner] = useState<string>('');
  const [aiResponse, setAiResponse] =
    useState<HeraCloudflareFreeAiResponse | null>(null);

  const handleCalculateBridge = async () => {
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
  };

  useEffect(() => {
    void handleCalculateBridge();
  }, []);

  const handlePreviewTransition = (cueOutSeconds: number, title: string) => {
    try {
      Player.setCurrentTime(cueOutSeconds);
    } catch {
      // Fallback if no local audio stream is currently loaded
    }
    setPreviewStatus(
      `🎧 Pre-escuchando cruce de 15s: Cue 4 (Mix Out ${cueOutSeconds.toFixed(1)}s) ➔ Cue 1 (Intro) de "${title}"`,
    );
  };

  const handleBackendChange = (nextBackend: string) => {
    setSelectedBackend(nextBackend);
    if (nextBackend === 'lmstudio') {
      setCustomBaseUrl('http://localhost:1234/v1');
      setCustomModel('default');
    } else if (nextBackend === 'ollama') {
      setCustomBaseUrl('http://localhost:11434/v1');
      setCustomModel('llama3.3');
    } else if (nextBackend === 'cloudflare_free') {
      setCustomBaseUrl('');
      setCustomModel('@cf/meta/llama-3.1-8b-instruct-fp8-fast');
    }
  };

  const handleProbeModels = async () => {
    const probe = await probeLocalAiBackend(customBaseUrl || undefined);
    if (probe.reachable && probe.models.length > 0) {
      setCustomModel(probe.models[0]);
      setProbeBanner(
        `✅ Conectado a ${probe.backend} (${probe.base_url}) · Modelos activos: ${probe.models.join(', ')}`,
      );
    } else {
      setProbeBanner(
        `⚠️ No se detectó servidor activo en ${probe.base_url} (usando fallback determinista o verifica LM Studio :1234)`,
      );
    }
  };

  const handleAskCopilot = async () => {
    const res = await chatWithCloudflareFreeAi(chatPrompt, 'DaviDJ', {
      backend: selectedBackend,
      baseUrl: customBaseUrl || undefined,
      model: customModel || undefined,
    });
    setAiResponse(res);
  };

  return (
    <View>
      <div {...stylex.props(styles.container)}>
        <div {...stylex.props(styles.headerRow)}>
          <div>
            <h2 {...stylex.props(styles.title)}>
              🌍 Puente Armónico Inteligente P2P & Copiloto IA Gratuito
            </h2>
            <p {...stylex.props(styles.subtitle)}>
              Encuentra las 2 canciones puente exactas en el enjambre P2P,
              pre-escucha el cruce Cue 4 ➔ Cue 1 y descárgalas directo a tu
              disco duro
            </p>
          </div>
          <div {...stylex.props(styles.bridgeActions)}>
            <span {...stylex.props(styles.camelotBadge)}>
              ⚡ Saldo P2P Boost: {turboCreditsMb.toFixed(1)} MB (Ganado 1:1.5
              sembrando en el enjambre)
            </span>
            <button
              type="button"
              onClick={handleCalculateBridge}
              {...stylex.props(styles.primaryBtn)}
            >
              🔍 Recalcular Puente Armónico (2-Opt + Camelot)
            </button>
          </div>
        </div>

        <div {...stylex.props(styles.endpointsGrid)}>
          <div {...stylex.props(styles.endpointCard)}>
            <span {...stylex.props(styles.badgeOrigin)}>
              🟢 PISTA ORIGEN (DECK A)
            </span>
            <div {...stylex.props(styles.formRow)}>
              <input
                type="text"
                value={originArtist}
                onChange={(e) => setOriginArtist(e.target.value)}
                placeholder="Artista Origen"
                {...stylex.props(styles.input)}
              />
              <input
                type="text"
                value={originTitle}
                onChange={(e) => setOriginTitle(e.target.value)}
                placeholder="Título Origen"
                {...stylex.props(styles.input)}
              />
            </div>
            <div {...stylex.props(styles.formRow)}>
              <input
                type="number"
                value={originBpm}
                onChange={(e) => setOriginBpm(Number(e.target.value))}
                {...stylex.props(styles.inputSmall)}
              />
              <input
                type="text"
                value={originCamelot}
                onChange={(e) => setOriginCamelot(e.target.value)}
                {...stylex.props(styles.inputSmall)}
              />
            </div>
          </div>

          <div {...stylex.props(styles.endpointCard)}>
            <span {...stylex.props(styles.badgeTarget)}>
              🔴 PISTA DESTINO (DECK B)
            </span>
            <div {...stylex.props(styles.formRow)}>
              <input
                type="text"
                value={targetArtist}
                onChange={(e) => setTargetArtist(e.target.value)}
                placeholder="Artista Destino"
                {...stylex.props(styles.input)}
              />
              <input
                type="text"
                value={targetTitle}
                onChange={(e) => setTargetTitle(e.target.value)}
                placeholder="Título Destino"
                {...stylex.props(styles.input)}
              />
            </div>
            <div {...stylex.props(styles.formRow)}>
              <input
                type="number"
                value={targetBpm}
                onChange={(e) => setTargetBpm(Number(e.target.value))}
                {...stylex.props(styles.inputSmall)}
              />
              <input
                type="text"
                value={targetCamelot}
                onChange={(e) => setTargetCamelot(e.target.value)}
                {...stylex.props(styles.inputSmall)}
              />
            </div>
          </div>
        </div>

        {bridgeResult && (
          <div {...stylex.props(styles.bridgeChain)}>
            {bridgeResult.bridge_steps.map((step) => (
              <div key={step.track_id} {...stylex.props(styles.bridgeCard)}>
                <div {...stylex.props(styles.bridgeHeader)}>
                  <strong>
                    🔗 Puente P2P #{step.step_number}: {step.artist} —{' '}
                    {step.title}
                  </strong>
                  <span {...stylex.props(styles.camelotBadge)}>
                    {step.bpm.toFixed(1)} BPM · {step.camelot} · Sinergia{' '}
                    {Math.round(step.synergy_score * 100)}%
                    {step.p2p_boost_free_promo
                      ? ' · 🚀 PROMO POOL (P2P Boost 0 MB)'
                      : ''}
                  </span>
                </div>
                <p {...stylex.props(styles.tipText)}>{step.transition_tip}</p>
                <div {...stylex.props(styles.bridgeActions)}>
                  <button
                    type="button"
                    onClick={() =>
                      handlePreviewTransition(step.cue_out_seconds, step.title)
                    }
                    {...stylex.props(styles.secondaryBtn)}
                  >
                    🎧 Pre-escuchar Transición (Cue 4 ➔ Cue 1 · 15s)
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      setPreviewStatus(
                        `📥 Descarga P2P Estándar ($0.00): "${step.artist} - ${step.title}" (${step.hera_seal}) desde nodo ${step.owner_dj} ➔ quarantine/ ➔ library/`,
                      )
                    }
                    {...stylex.props(styles.secondaryBtn)}
                  >
                    📥 P2P Gratis ({step.owner_dj})
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      const cost = step.p2p_boost_free_promo
                        ? 0
                        : (step.boost_credit_cost_mb ?? 38);
                      if (cost > 0 && turboCreditsMb >= cost) {
                        setTurboCreditsMb((prev) => prev - cost);
                      }
                      setPreviewStatus(
                        step.p2p_boost_free_promo
                          ? `🚀 P2P Boost Promo Pool (0 MB Créditos · Patrocinado por el Productor): "${step.artist} - ${step.title}" descargado a máxima velocidad (zk-ctr-sha256-v1) ➔ library/`
                          : `⚡ P2P Boost Activado (-${cost} MB Créditos Turbo · Restan ${Math.max(0, turboCreditsMb - cost).toFixed(1)} MB): "${step.artist} - ${step.title}" descargado desde caché R2 Zero-Knowledge ➔ library/`,
                      );
                    }}
                    {...stylex.props(styles.primaryBtn)}
                  >
                    {step.p2p_boost_free_promo
                      ? '🚀 P2P Boost Gratis (Promo Pool · 0 MB)'
                      : `⚡ P2P Boost (${step.boost_credit_cost_mb ?? 38} MB Créditos)`}
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

        {previewStatus && (
          <div {...stylex.props(styles.statusBanner)}>{previewStatus}</div>
        )}

        <div {...stylex.props(styles.copilotBox)}>
          <div {...stylex.props(styles.bridgeHeader)}>
            <strong>
              🤖 HERA Agentic Copilot (ReAct Multi-Turn + 20 Herramientas MCP ·
              Local LM Studio / Ollama / Cloud Free)
            </strong>
            <span {...stylex.props(styles.subtitle)}>
              Motor activo: {selectedBackend} · Endpoint:{' '}
              {customBaseUrl || 'Cloudflare Workers AI'}
            </span>
          </div>
          <div {...stylex.props(styles.formRow)}>
            <select
              value={selectedBackend}
              onChange={(e) => handleBackendChange(e.target.value)}
              {...stylex.props(styles.input)}
            >
              <option value="lmstudio">
                🖥️ LM Studio (Local :1234 / LAN OpenAI Server)
              </option>
              <option value="ollama">💻 Ollama (Local :11434)</option>
              <option value="cloudflare_free">
                ⚡ Hera Cloud Free (Workers AI · 10,000 Neurons/día a $0.00)
              </option>
              <option value="openrouter">🌐 OpenRouter (:free & Pro)</option>
              <option value="gemini">✨ Google Gemini</option>
              <option value="openai">🧠 OpenAI</option>
              <option value="custom">🔌 Custom OpenAI Endpoint</option>
            </select>
            <input
              type="text"
              value={customBaseUrl}
              onChange={(e) => setCustomBaseUrl(e.target.value)}
              placeholder="http://localhost:1234/v1 o IP LAN"
              {...stylex.props(styles.input)}
            />
            <input
              type="text"
              value={customModel}
              onChange={(e) => setCustomModel(e.target.value)}
              placeholder="Modelo (ej. qwen2.5-coder-14b)"
              {...stylex.props(styles.input)}
            />
            <button
              type="button"
              onClick={handleProbeModels}
              {...stylex.props(styles.secondaryBtn)}
            >
              🔍 Detectar LM Studio / Modelos
            </button>
          </div>
          {probeBanner && (
            <div {...stylex.props(styles.statusBanner)}>{probeBanner}</div>
          )}
          <div {...stylex.props(styles.formRow)}>
            <input
              type="text"
              value={chatPrompt}
              onChange={(e) => setChatPrompt(e.target.value)}
              {...stylex.props(styles.input)}
            />
            <button
              type="button"
              onClick={handleAskCopilot}
              {...stylex.props(styles.primaryBtn)}
            >
              🤖 Consultar Copiloto
            </button>
          </div>
          {aiResponse && (
            <div {...stylex.props(styles.aiReply)}>
              {aiResponse.tool_calls_trace &&
                aiResponse.tool_calls_trace.length > 0 && (
                  <div {...stylex.props(styles.endpointCard)}>
                    {aiResponse.tool_calls_trace.map((step, idx) => (
                      <small
                        key={`${step.step}-${step.tool_name}-${idx}`}
                        {...stylex.props(styles.subtitle)}
                      >
                        🛠️ Paso {step.step}: {step.tool_name} -&gt;{' '}
                        {step.result_preview}
                      </small>
                    ))}
                  </div>
                )}
              <p {...stylex.props(styles.tipText)}>
                {aiResponse.assistant_reply}
              </p>
              <small {...stylex.props(styles.subtitle)}>
                Motor: {aiResponse.backend_used} ({aiResponse.model_used}) ·
                Iteraciones ReAct: {aiResponse.iterations_executed ?? 1} ·
                Neurons consumidas: {aiResponse.neurons_consumed_this_turn} ·
                Restantes hoy: {aiResponse.daily_neurons_remaining.toFixed(1)} ·
                Costo: ${aiResponse.estimated_cost_usd.toFixed(2)}
              </small>
            </div>
          )}
        </div>
      </div>
    </View>
  );
}

const styles = stylex.create({
  container: {
    padding: '20px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '14px',
    overflowY: 'auto',
    height: '100%',
  },
  headerRow: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    flexWrap: 'wrap',
    rowGap: '12px',
    columnGap: '12px',
  },
  title: {
    margin: 0,
    fontSize: '18px',
  },
  subtitle: {
    margin: 0,
    fontSize: '12px',
    color: 'var(--text-muted)',
  },
  endpointsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))',
    rowGap: '12px',
    columnGap: '12px',
  },
  endpointCard: {
    padding: '12px',
    borderRadius: '8px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--tracks-bg-even)',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  badgeOrigin: {
    fontSize: '11px',
    fontWeight: 'bold',
    color: '#28E214',
  },
  badgeTarget: {
    fontSize: '11px',
    fontWeight: 'bold',
    color: '#FF2D2D',
  },
  formRow: {
    display: 'flex',
    rowGap: '8px',
    columnGap: '8px',
  },
  input: {
    paddingBlock: '5px',
    paddingInline: '8px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--header-bg)',
    color: 'inherit',
    flexGrow: 1,
    fontSize: '12px',
  },
  inputSmall: {
    paddingBlock: '5px',
    paddingInline: '8px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--header-bg)',
    color: 'inherit',
    width: '90px',
    fontSize: '12px',
  },
  bridgeChain: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(360px, 1fr))',
    rowGap: '12px',
    columnGap: '12px',
  },
  bridgeCard: {
    padding: '12px',
    borderRadius: '8px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--main-color)',
    backgroundColor: 'rgba(139, 92, 246, 0.08)',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  bridgeHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    flexWrap: 'wrap',
    rowGap: '6px',
    columnGap: '6px',
    fontSize: '12px',
  },
  camelotBadge: {
    paddingBlock: '2px',
    paddingInline: '6px',
    borderRadius: '4px',
    backgroundColor: 'rgba(139, 92, 246, 0.22)',
    fontSize: '11px',
    fontWeight: 'bold',
  },
  tipText: {
    margin: 0,
    fontSize: '12px',
  },
  bridgeActions: {
    display: 'flex',
    rowGap: '8px',
    columnGap: '8px',
    flexWrap: 'wrap',
  },
  primaryBtn: {
    paddingBlock: '6px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--main-color)',
    backgroundColor: 'rgba(139, 92, 246, 0.2)',
    color: 'inherit',
    cursor: 'pointer',
    fontSize: '12px',
    fontWeight: 'bold',
  },
  secondaryBtn: {
    paddingBlock: '6px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--header-bg)',
    color: 'inherit',
    cursor: 'pointer',
    fontSize: '12px',
  },
  statusBanner: {
    paddingBlock: '8px',
    paddingInline: '12px',
    borderRadius: '6px',
    backgroundColor: 'rgba(40, 226, 20, 0.14)',
    fontSize: '12px',
  },
  copilotBox: {
    padding: '12px',
    borderRadius: '8px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--tracks-bg-even)',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  aiReply: {
    padding: '8px',
    borderRadius: '6px',
    backgroundColor: 'var(--header-bg)',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '4px',
  },
});

import * as stylex from '@stylexjs/stylex';
import { useEffect, useRef, useState } from 'react';

import {
  chatWithCloudflareFreeAi,
  type CommanderAuthorityMode,
  type HarmonicBridgeStep,
  scanAiConnectivityOnboarding,
} from '../../lib/bridge-hera';
import Icon from '../Icon';
import TransitionAssistantPopover from './TransitionAssistantPopover';

export type TransitionCommandDockProps = {
  activeSteps: Array<HarmonicBridgeStep>;
  originBpm: number;
  targetBpm: number;
  onApplyCustomSteps: (steps: Array<HarmonicBridgeStep>) => void;
};

async function runCommandAdjustmentSafe(params: {
  promptToSend: string;
  activeSteps: Array<HarmonicBridgeStep>;
  originBpm: number;
  targetBpm: number;
  onApplyCustomSteps: (steps: Array<HarmonicBridgeStep>) => void;
  activeBackend: string;
  baseUrl: string | undefined;
  commanderMode: CommanderAuthorityMode;
}): Promise<string> {
  try {
    const lower = params.promptToSend.toLowerCase();
    if (lower.includes('energ') || lower.includes('arriba')) {
      params.onApplyCustomSteps(
        params.activeSteps.map((step, i) => ({
          ...step,
          bpm: Math.round((step.bpm + (i === 0 ? 1.5 : 2.0)) * 10) / 10,
        })),
      );
    } else if (lower.includes('suave') || lower.includes('lenta')) {
      params.onApplyCustomSteps(
        params.activeSteps.map((step, i) => ({
          ...step,
          bpm:
            Math.round(
              (params.originBpm +
                ((params.targetBpm - params.originBpm) * (i + 1)) /
                  (params.activeSteps.length + 1)) *
                10,
            ) / 10,
        })),
      );
    }

    await chatWithCloudflareFreeAi(params.promptToSend, 'DaviDJ', {
      backend: params.activeBackend,
      baseUrl: params.baseUrl,
      commanderMode: params.commanderMode,
    });

    return `Ajuste aplicado: ${params.promptToSend}`;
  } catch {
    return `Ajuste local aplicado: ${params.promptToSend}`;
  }
}

export default function TransitionCommandDock({
  activeSteps,
  originBpm,
  targetBpm,
  onApplyCustomSteps,
}: TransitionCommandDockProps) {
  const [chatPrompt, setChatPrompt] = useState('');
  const [quietNote, setQuietNote] = useState<string>('');
  const [isAiLoading, setIsAiLoading] = useState<boolean>(false);
  const [isAssistantOpen, setIsAssistantOpen] = useState<boolean>(false);
  const [showAdvancedUrl, setShowAdvancedUrl] = useState<boolean>(false);
  const [connectionScope, setConnectionScope] = useState<'cloud' | 'offline'>(
    'cloud',
  );
  const [commanderMode, setCommanderMode] =
    useState<CommanderAuthorityMode>('executive_boss');
  const [customBaseUrl, setCustomBaseUrl] = useState<string>(
    'http://127.0.0.1:11434/v1',
  );

  const offlineBackendIdRef = useRef<string>('ollama');
  const noteTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const popoverRef = useRef<HTMLDivElement | null>(null);

  const flashQuietNote = (message: string) => {
    if (noteTimerRef.current) clearTimeout(noteTimerRef.current);
    setQuietNote(message);
    noteTimerRef.current = setTimeout(() => {
      setQuietNote('');
    }, 2600);
  };

  useEffect(() => {
    return () => {
      if (noteTimerRef.current) clearTimeout(noteTimerRef.current);
    };
  }, []);

  useEffect(() => {
    if (!isAssistantOpen) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsAssistantOpen(false);
      }
    };
    const onPointerDown = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node)
      ) {
        setIsAssistantOpen(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('mousedown', onPointerDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('mousedown', onPointerDown);
    };
  }, [isAssistantOpen]);

  useEffect(() => {
    void scanAiConnectivityOnboarding(commanderMode)
      .then((rep) => {
        const localServer = rep.local_servers.find((s) => s.reachable);
        if (localServer) {
          offlineBackendIdRef.current = localServer.id;
        }
      })
      .catch(() => {});
  }, [commanderMode]);

  const handleSendAdjustment = async (presetText?: string) => {
    const promptToSend = (presetText ?? chatPrompt).trim();
    if (!promptToSend || isAiLoading) return;
    if (!presetText) {
      setChatPrompt('');
    }
    setIsAiLoading(true);
    const activeBackend =
      connectionScope === 'cloud'
        ? 'cloudflare_free'
        : showAdvancedUrl
          ? 'custom'
          : offlineBackendIdRef.current;

    const note = await runCommandAdjustmentSafe({
      promptToSend,
      activeSteps,
      originBpm,
      targetBpm,
      onApplyCustomSteps,
      activeBackend,
      baseUrl:
        connectionScope === 'offline' && showAdvancedUrl
          ? customBaseUrl
          : undefined,
      commanderMode,
    });
    flashQuietNote(note);
    setIsAiLoading(false);
  };

  return (
    <footer {...stylex.props(styles.commandFooter)}>
      {quietNote && !isAssistantOpen && (
        <div {...stylex.props(styles.quietStatusNote)}>{quietNote}</div>
      )}

      <div {...stylex.props(styles.commandDockRow)} ref={popoverRef}>
        <div {...stylex.props(styles.spotlightBar)}>
          <input
            type="text"
            value={chatPrompt}
            onChange={(e) => setChatPrompt(e.target.value)}
            onKeyDown={(e) => {
              if (e.nativeEvent.isComposing) return;
              if (e.key === 'Enter') {
                void handleSendAdjustment();
              }
            }}
            placeholder={
              isAiLoading ? 'Ajustando ruta...' : 'Ajustar transición...'
            }
            aria-label="Ajustar transición"
            {...stylex.props(styles.spotlightInput)}
          />

          <div {...stylex.props(styles.quickPills)}>
            <button
              type="button"
              disabled={isAiLoading}
              onClick={() => void handleSendAdjustment('Más suave')}
              {...stylex.props(styles.quickPill)}
            >
              Más suave
            </button>
            <button
              type="button"
              disabled={isAiLoading}
              onClick={() => void handleSendAdjustment('Más energía')}
              {...stylex.props(styles.quickPill)}
            >
              Más energía
            </button>
            <button
              type="button"
              disabled={isAiLoading}
              onClick={() => void handleSendAdjustment('Sin voces')}
              {...stylex.props(styles.quickPill)}
            >
              Sin voces
            </button>
          </div>
        </div>

        <button
          type="button"
          aria-label="Configurar asistente"
          aria-expanded={isAssistantOpen}
          title="Asistente"
          onClick={() => setIsAssistantOpen((prev) => !prev)}
          {...stylex.props(
            styles.assistantTriggerBtn,
            isAssistantOpen && styles.assistantTriggerBtnActive,
          )}
        >
          <Icon name="sliders" size={16} />
        </button>

        {isAssistantOpen && (
          <TransitionAssistantPopover
            connectionScope={connectionScope}
            commanderMode={commanderMode}
            showAdvancedUrl={showAdvancedUrl}
            customBaseUrl={customBaseUrl}
            onConnectionScopeChange={setConnectionScope}
            onCommanderModeChange={setCommanderMode}
            onToggleAdvancedUrl={() => setShowAdvancedUrl((v) => !v)}
            onCustomBaseUrlChange={setCustomBaseUrl}
          />
        )}
      </div>
    </footer>
  );
}

const styles = stylex.create({
  commandFooter: {
    flexShrink: 0,
    paddingTop: '10px',
    paddingBottom: '18px',
    paddingInline: '24px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1c222d',
    backgroundColor: '#0c0e12',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    rowGap: '6px',
  },
  quietStatusNote: {
    fontSize: '11.5px',
    color: '#2dd4bf',
  },
  commandDockRow: {
    position: 'relative',
    width: '100%',
    maxWidth: '640px',
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  spotlightBar: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '6px',
    paddingLeft: '14px',
    paddingRight: '8px',
    borderRadius: '10px',
    backgroundColor: '#131820',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
  },
  spotlightInput: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    borderWidth: 0,
    outline: 'none',
    backgroundColor: 'transparent',
    color: '#edeff3',
    fontSize: '13px',
    paddingBlock: '4px',
  },
  quickPills: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  quickPill: {
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '999px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#262f3d',
    backgroundColor: '#1b222d',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  assistantTriggerBtn: {
    width: '36px',
    height: '36px',
    borderRadius: '9px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    backgroundColor: '#151a22',
    color: '#8e98a8',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    flexShrink: 0,
  },
  assistantTriggerBtnActive: {
    borderColor: 'rgba(20, 184, 166, 0.45)',
    color: '#edeff3',
    backgroundColor: '#1a212c',
  },
});

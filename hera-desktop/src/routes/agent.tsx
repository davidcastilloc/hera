import * as stylex from '@stylexjs/stylex';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import AgentComposer from '../components/AgentComposer';
import AgentToolStepRow from '../components/AgentToolStepRow';
import Icon from '../components/Icon';
import View from '../elements/View';
import { useAgentStore } from '../lib/agent-store';
import type { AgentFollowupAction } from '../lib/bridge-agent';
import { useMiniModeStore } from '../lib/mini-mode-store';
import player from '../lib/player';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/agent')({
  component: ViewAgentWorkspace,
});

const SUGGESTION_PILLS: Array<{ label: string; prompt: string }> = [
  {
    label: 'Armar playlist armónica',
    prompt:
      'Arma una playlist «Warmup Melódico» ordenada por llave Camelot y mándala a VirtualDJ',
  },
  {
    label: 'Enviar selección a VirtualDJ',
    prompt: 'Sincroniza mis mejores pistas en la carpeta en vivo de VirtualDJ',
  },
  {
    label: 'Inspeccionar carpeta musical',
    prompt:
      'Inspecciona los archivos recientes en mi directorio temporal y resume formatos disponibles',
  },
  {
    label: 'Diagnosticar audio y sistema',
    prompt:
      'Ejecuta un diagnóstico rápido de mi biblioteca local, VirtualDJ y estado del sistema',
  },
];

const TRASH_MINI_SVG = (
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.7"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3" />
  </svg>
);

function renderFormattedLine(line: string, idx: number) {
  const trimmed = line.trim();
  if (!trimmed) {
    return <div key={idx} {...stylex.props(styles.lineSpacer)} />;
  }

  const isBullet = trimmed.startsWith('- ') || trimmed.startsWith('• ');
  const body = isBullet ? trimmed.slice(2) : line;

  const segments = body.split(/(\*\*[^*]+\*\*|`[^`]+`)/g);
  const children = segments.map((part, pIdx) => {
    if (part.startsWith('**') && part.endsWith('**') && part.length > 4) {
      return (
        <strong key={pIdx} {...stylex.props(styles.inlineStrong)}>
          {part.slice(2, -2)}
        </strong>
      );
    }
    if (part.startsWith('`') && part.endsWith('`') && part.length > 2) {
      return (
        <code key={pIdx} {...stylex.props(styles.inlineCode)}>
          {part.slice(1, -1)}
        </code>
      );
    }
    return part;
  });

  if (isBullet) {
    return (
      <div key={idx} {...stylex.props(styles.bulletRow)}>
        <span {...stylex.props(styles.bulletDot)}>•</span>
        <span>{children}</span>
      </div>
    );
  }

  return (
    <p key={idx} {...stylex.props(styles.replyParagraph)}>
      {children}
    </p>
  );
}

function ViewAgentWorkspace() {
  const navigate = useNavigate();
  const tracks = useLibraryStore((s) => s.tracks);
  const enterMiniCard = useMiniModeStore((s) => s.enterMiniCard);

  const runtimeStatus = useAgentStore((s) => s.runtimeStatus);
  const sessions = useAgentStore((s) => s.sessions);
  const activeSessionId = useAgentStore((s) => s.activeSessionId);
  const messagesBySession = useAgentStore((s) => s.messagesBySession);
  const runningSessionIds = useAgentStore((s) => s.runningSessionIds);
  const backendId = useAgentStore((s) => s.backendId);
  const executionPolicy = useAgentStore((s) => s.executionPolicy);
  const draftText = useAgentStore((s) => s.draftText);
  const draftAttachments = useAgentStore((s) => s.draftAttachments);

  const bootstrap = useAgentStore((s) => s.bootstrap);
  const selectSession = useAgentStore((s) => s.selectSession);
  const startNewSession = useAgentStore((s) => s.startNewSession);
  const removeSession = useAgentStore((s) => s.removeSession);
  const setBackendId = useAgentStore((s) => s.setBackendId);
  const setExecutionPolicy = useAgentStore((s) => s.setExecutionPolicy);
  const setDraftText = useAgentStore((s) => s.setDraftText);
  const addDraftAttachment = useAgentStore((s) => s.addDraftAttachment);
  const removeDraftAttachment = useAgentStore((s) => s.removeDraftAttachment);
  const sendMessage = useAgentStore((s) => s.sendMessage);
  const stopSessionTurn = useAgentStore((s) => s.stopSessionTurn);
  const approveStep = useAgentStore((s) => s.approveStep);

  const [sessionMenuOpen, setSessionMenuOpen] = useState(false);
  const threadEndRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  const activeSession = useMemo(
    () => sessions.find((s) => s.id === activeSessionId) ?? null,
    [activeSessionId, sessions],
  );

  const activeMessages = useMemo(
    () => (activeSessionId ? (messagesBySession[activeSessionId] ?? []) : []),
    [activeSessionId, messagesBySession],
  );

  const isRunning = activeSessionId
    ? runningSessionIds.includes(activeSessionId)
    : false;

  useEffect(() => {
    if (activeMessages.length > 0) {
      threadEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [activeMessages]);

  const handleFollowupAction = useCallback(
    (act: AgentFollowupAction) => {
      if (act.kind === 'play_track') {
        const target = tracks.find((t) => t.id === act.target_id) ?? tracks[0];
        if (target) {
          void player.start(tracks, target.id, { type: 'library' });
        }
        return;
      }
      if (act.kind === 'open_playlist') {
        void navigate({
          to: '/playlists/$playlistID',
          params: { playlistID: act.target_id },
        });
        return;
      }
      if (act.kind === 'open_library') {
        void navigate({ to: '/library' });
        return;
      }
      if (act.kind === 'mini_mode') {
        void enterMiniCard();
      }
    },
    [enterMiniCard, navigate, tracks],
  );

  const toolsCount = runtimeStatus?.tools_count ?? 42;
  const agyInstalled = Boolean(runtimeStatus?.agy_installed);
  const engineStatusLabel =
    backendId === 'cli_agy' && agyInstalled
      ? `agy conectado · ${toolsCount} tools`
      : `Motor activo · ${toolsCount} tools`;

  return (
    <View>
      <div {...stylex.props(styles.pageShell)}>
        {/* Minimalist 44px Header */}
        <header {...stylex.props(styles.topBar)}>
          <div {...stylex.props(styles.sessionLeftGroup)}>
            <div {...stylex.props(styles.dropdownAnchor)}>
              <button
                type="button"
                aria-expanded={sessionMenuOpen}
                onClick={() => setSessionMenuOpen((prev) => !prev)}
                {...stylex.props(styles.sessionPickerBtn)}
              >
                <span {...stylex.props(styles.sessionTitleLabel)}>
                  {activeSession ? activeSession.title : 'Nueva sesión'}
                </span>
                <Icon name="chevronDown" size={12} />
              </button>

              {sessionMenuOpen ? (
                <div
                  role="menu"
                  aria-label="Sesiones recientes"
                  {...stylex.props(styles.sessionDropdown)}
                >
                  {sessions.length === 0 ? (
                    <div {...stylex.props(styles.emptySessionsHint)}>
                      Sin sesiones guardadas aún
                    </div>
                  ) : (
                    sessions.map((sess) => (
                      <div
                        key={sess.id}
                        {...stylex.props(
                          styles.sessionRowItem,
                          sess.id === activeSessionId &&
                            styles.sessionRowItemActive,
                        )}
                      >
                        <button
                          type="button"
                          onClick={() => {
                            setSessionMenuOpen(false);
                            void selectSession(sess.id);
                          }}
                          {...stylex.props(styles.sessionRowSelectBtn)}
                        >
                          <span {...stylex.props(styles.sessionRowTitle)}>
                            {sess.title}
                          </span>
                          <span {...stylex.props(styles.sessionRowCount)}>
                            {sess.message_count}
                          </span>
                        </button>
                        <button
                          type="button"
                          aria-label={`Eliminar sesión ${sess.title}`}
                          onClick={() => void removeSession(sess.id)}
                          {...stylex.props(styles.sessionDeleteBtn)}
                        >
                          {TRASH_MINI_SVG}
                        </button>
                      </div>
                    ))
                  )}
                </div>
              ) : null}
            </div>

            <button
              type="button"
              onClick={() => {
                setSessionMenuOpen(false);
                startNewSession();
              }}
              {...stylex.props(styles.newSessionBtn)}
            >
              <Icon name="plus" size={12} />
              <span>Nueva</span>
            </button>
          </div>

          <div {...stylex.props(styles.statusPill)}>
            <span {...stylex.props(styles.statusDot)} />
            <span>{engineStatusLabel}</span>
          </div>
        </header>

        {activeMessages.length === 0 ? (
          /* Clean Centered Empty State */
          <div {...stylex.props(styles.emptyStage)}>
            <div {...stylex.props(styles.emptyCenterColumn)}>
              <h1 {...stylex.props(styles.emptyHeading)}>¿Qué hacemos hoy?</h1>

              <AgentComposer
                value={draftText}
                attachments={draftAttachments}
                backendId={backendId}
                agyInstalled={agyInstalled}
                executionPolicy={executionPolicy}
                isRunning={isRunning}
                focusOnMount
                onChange={setDraftText}
                onAddAttachment={addDraftAttachment}
                onRemoveAttachment={removeDraftAttachment}
                onBackendChange={setBackendId}
                onPolicyChange={setExecutionPolicy}
                onSubmit={() => void sendMessage()}
                onStop={() => void stopSessionTurn()}
              />

              <div {...stylex.props(styles.suggestionsGrid)}>
                {SUGGESTION_PILLS.map((pill) => (
                  <button
                    key={pill.label}
                    type="button"
                    onClick={() => void sendMessage(pill.prompt)}
                    {...stylex.props(styles.suggestionPill)}
                  >
                    {pill.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        ) : (
          /* Active Conversation Thread */
          <div {...stylex.props(styles.activeStage)}>
            <div {...stylex.props(styles.threadScroll)}>
              <div {...stylex.props(styles.threadColumn)}>
                {activeMessages.map((msg) =>
                  msg.role === 'user' ? (
                    <div key={msg.id} {...stylex.props(styles.userRow)}>
                      <div {...stylex.props(styles.userBubble)}>
                        {msg.attachments.length > 0 ? (
                          <div {...stylex.props(styles.userAttRow)}>
                            {msg.attachments.map((att) => (
                              <span
                                key={att.id}
                                {...stylex.props(styles.userAttTag)}
                              >
                                @{att.label}
                              </span>
                            ))}
                          </div>
                        ) : null}
                        {msg.content ? (
                          <div {...stylex.props(styles.userText)}>
                            {msg.content}
                          </div>
                        ) : null}
                      </div>
                    </div>
                  ) : (
                    <div key={msg.id} {...stylex.props(styles.assistantBlock)}>
                      {msg.tool_steps.length > 0 ? (
                        <div {...stylex.props(styles.stepsStack)}>
                          {msg.tool_steps.map((step) => (
                            <AgentToolStepRow
                              key={step.id}
                              step={step}
                              onApprove={(id, ok) => void approveStep(id, ok)}
                            />
                          ))}
                        </div>
                      ) : null}

                      {msg.content ? (
                        <div {...stylex.props(styles.assistantProse)}>
                          {msg.content
                            .split('\n')
                            .map((line, idx) => renderFormattedLine(line, idx))}
                        </div>
                      ) : msg.status === 'streaming' &&
                        msg.tool_steps.length === 0 ? (
                        <div {...stylex.props(styles.thinkingRow)}>
                          Procesando solicitud…
                        </div>
                      ) : null}

                      {msg.actions && msg.actions.length > 0 ? (
                        <div {...stylex.props(styles.followupRow)}>
                          {msg.actions.map((act) => (
                            <button
                              key={`${act.kind}-${act.target_id}`}
                              type="button"
                              onClick={() => handleFollowupAction(act)}
                              {...stylex.props(styles.followupBtn)}
                            >
                              <Icon
                                name={
                                  act.kind === 'play_track'
                                    ? 'play'
                                    : 'crateList'
                                }
                                size={12}
                              />
                              <span>{act.label}</span>
                            </button>
                          ))}
                        </div>
                      ) : null}
                    </div>
                  ),
                )}
                <div ref={threadEndRef} />
              </div>
            </div>

            <div {...stylex.props(styles.composerDock)}>
              <div {...stylex.props(styles.composerDockInner)}>
                <AgentComposer
                  value={draftText}
                  attachments={draftAttachments}
                  backendId={backendId}
                  agyInstalled={agyInstalled}
                  executionPolicy={executionPolicy}
                  isRunning={isRunning}
                  onChange={setDraftText}
                  onAddAttachment={addDraftAttachment}
                  onRemoveAttachment={removeDraftAttachment}
                  onBackendChange={setBackendId}
                  onPolicyChange={setExecutionPolicy}
                  onSubmit={() => void sendMessage()}
                  onStop={() => void stopSessionTurn()}
                />
              </div>
            </div>
          </div>
        )}
      </div>
    </View>
  );
}

const styles = stylex.create({
  pageShell: {
    display: 'flex',
    flexDirection: 'column',
    width: '100%',
    height: '100%',
    minHeight: 0,
    minWidth: 0,
    backgroundColor: '#0c0e12',
    color: '#edeff3',
  },
  topBar: {
    height: '44px',
    flexShrink: 0,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingInline: '20px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    minWidth: 0,
  },
  sessionLeftGroup: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    minWidth: 0,
  },
  dropdownAnchor: {
    position: 'relative',
  },
  sessionPickerBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '7px',
    height: '28px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: {
      default: '#10141b',
      ':hover': '#161c26',
    },
    color: '#edeff3',
    fontSize: '12.5px',
    fontWeight: 600,
    cursor: 'pointer',
    maxWidth: '280px',
  },
  sessionTitleLabel: {
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  sessionDropdown: {
    position: 'absolute',
    top: 'calc(100% + 6px)',
    left: 0,
    zIndex: 40,
    width: '280px',
    maxHeight: '260px',
    overflowY: 'auto',
    backgroundColor: '#12161e',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    borderRadius: '8px',
    paddingBlock: '4px',
    paddingInline: '4px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    boxShadow: '0 10px 24px rgba(0, 0, 0, 0.45)',
  },
  emptySessionsHint: {
    paddingBlock: '10px',
    paddingInline: '10px',
    fontSize: '12px',
    color: '#8e98a8',
  },
  sessionRowItem: {
    display: 'flex',
    alignItems: 'center',
    borderRadius: '6px',
    backgroundColor: {
      default: 'transparent',
      ':hover': 'rgba(237, 239, 243, 0.04)',
    },
  },
  sessionRowItemActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
  },
  sessionRowSelectBtn: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
    flexGrow: 1,
    minWidth: 0,
    paddingBlock: '7px',
    paddingInline: '8px',
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#edeff3',
    fontSize: '12.5px',
    textAlign: 'left',
    cursor: 'pointer',
  },
  sessionRowTitle: {
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  sessionRowCount: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  sessionDeleteBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '26px',
    height: '26px',
    borderWidth: 0,
    borderRadius: '5px',
    backgroundColor: 'transparent',
    color: {
      default: '#647084',
      ':hover': '#f87171',
    },
    cursor: 'pointer',
    flexShrink: 0,
  },
  newSessionBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    height: '28px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: {
      default: 'transparent',
      ':hover': '#141922',
    },
    color: '#a3adbd',
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  statusPill: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '7px',
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  statusDot: {
    width: '6px',
    height: '6px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
  },
  emptyStage: {
    flexGrow: 1,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    paddingInline: '24px',
    paddingBottom: '28px',
    minHeight: 0,
    minWidth: 0,
    overflowY: 'auto',
  },
  emptyCenterColumn: {
    width: '100%',
    maxWidth: '680px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '18px',
    minWidth: 0,
  },
  emptyHeading: {
    margin: 0,
    fontSize: '24px',
    fontWeight: 600,
    letterSpacing: '-0.02em',
    color: '#edeff3',
    textAlign: 'center',
  },
  suggestionsGrid: {
    display: 'flex',
    flexWrap: 'wrap',
    justifyContent: 'center',
    rowGap: '8px',
    columnGap: '8px',
  },
  suggestionPill: {
    paddingBlock: '7px',
    paddingInline: '12px',
    borderRadius: '999px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: {
      default: '#12161e',
      ':hover': '#171d27',
    },
    color: {
      default: '#a3adbd',
      ':hover': '#edeff3',
    },
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
    transition: 'background-color 0.15s ease-out, color 0.15s ease-out',
  },
  activeStage: {
    flexGrow: 1,
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0,
    minWidth: 0,
  },
  threadScroll: {
    flexGrow: 1,
    overflowY: 'auto',
    paddingTop: '20px',
    paddingBottom: '12px',
    paddingInline: '24px',
    minHeight: 0,
    minWidth: 0,
  },
  threadColumn: {
    width: '100%',
    maxWidth: '720px',
    marginInline: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '20px',
    minWidth: 0,
  },
  userRow: {
    display: 'flex',
    justifyContent: 'flex-end',
    minWidth: 0,
  },
  userBubble: {
    maxWidth: '82%',
    paddingBlock: '9px',
    paddingInline: '13px',
    borderRadius: '12px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    backgroundColor: '#171c24',
    color: '#edeff3',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    minWidth: 0,
  },
  userAttRow: {
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '5px',
    columnGap: '5px',
  },
  userAttTag: {
    fontSize: '11.5px',
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
    paddingBlock: '1px',
    paddingInline: '6px',
    borderRadius: '4px',
  },
  userText: {
    fontSize: '13.5px',
    lineHeight: 1.5,
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-word',
  },
  assistantBlock: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
    minWidth: 0,
  },
  stepsStack: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    minWidth: 0,
  },
  assistantProse: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '4px',
    fontSize: '13.5px',
    lineHeight: 1.58,
    color: '#e2e6ee',
    minWidth: 0,
  },
  replyParagraph: {
    margin: 0,
    wordBreak: 'break-word',
  },
  lineSpacer: {
    height: '4px',
  },
  bulletRow: {
    display: 'flex',
    alignItems: 'flex-start',
    columnGap: '8px',
    paddingLeft: '4px',
  },
  bulletDot: {
    color: '#14b8a6',
    flexShrink: 0,
  },
  inlineStrong: {
    color: '#f4f6fa',
    fontWeight: 600,
  },
  inlineCode: {
    fontFamily:
      'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '12px',
    paddingBlock: '1px',
    paddingInline: '5px',
    borderRadius: '4px',
    backgroundColor: '#151a23',
    color: '#86efac',
  },
  thinkingRow: {
    fontSize: '12.5px',
    color: '#8e98a8',
    paddingLeft: '6px',
  },
  followupRow: {
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '8px',
    columnGap: '8px',
    paddingTop: '4px',
  },
  followupBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    height: '28px',
    paddingInline: '11px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.32)',
    backgroundColor: {
      default: 'rgba(20, 184, 166, 0.1)',
      ':hover': 'rgba(20, 184, 166, 0.18)',
    },
    color: '#2dd4bf',
    fontSize: '12px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  composerDock: {
    flexShrink: 0,
    paddingTop: '8px',
    paddingBottom: '16px',
    paddingInline: '24px',
    backgroundColor: '#0c0e12',
    minWidth: 0,
  },
  composerDockInner: {
    width: '100%',
    maxWidth: '720px',
    marginInline: 'auto',
    minWidth: 0,
  },
});

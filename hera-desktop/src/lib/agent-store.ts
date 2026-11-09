import { create } from 'zustand';

import {
  type AgentAttachment,
  type AgentMessageView,
  type AgentRuntimeStatus,
  type AgentSessionSummary,
  type AgentStreamEvent,
  type AgentToolStepView,
  type AgentUiSyncEvent,
  cancelAgentTurn,
  deleteAgentSession,
  getAgentRuntimeStatus,
  getAgentSessionMessages,
  listAgentSessions,
  listenAgentStream,
  listenAgentUiSync,
  resolveAgentApproval,
  startAgentTurn,
  updateAgentSession,
} from './bridge-agent';

export type AgentExecutionPolicy = 'auto' | 'confirm';

interface AgentStoreState {
  bootstrapped: boolean;
  runtimeStatus: AgentRuntimeStatus | null;
  sessions: AgentSessionSummary[];
  activeSessionId: string | null;
  messagesBySession: Record<string, AgentMessageView[]>;
  runningSessionIds: string[];
  backendId: string;
  executionPolicy: AgentExecutionPolicy;
  draftText: string;
  draftAttachments: AgentAttachment[];

  bootstrap: () => Promise<void>;
  selectSession: (sessionId: string) => Promise<void>;
  startNewSession: () => void;
  removeSession: (sessionId: string) => Promise<void>;
  setBackendId: (backendId: string) => void;
  setExecutionPolicy: (policy: AgentExecutionPolicy) => void;
  setDraftText: (text: string) => void;
  addDraftAttachment: (attachment: AgentAttachment) => void;
  removeDraftAttachment: (id: string) => void;
  clearDraftAttachments: () => void;
  sendMessage: (overrideText?: string) => Promise<void>;
  stopSessionTurn: (sessionId?: string | null) => Promise<void>;
  approveStep: (stepId: string, approved: boolean) => Promise<void>;
  applyStreamEvent: (event: AgentStreamEvent) => void;
}

function upsertStepInList(
  steps: AgentToolStepView[],
  incoming: AgentToolStepView,
): AgentToolStepView[] {
  const idx = steps.findIndex((s) => s.id === incoming.id);
  if (idx === -1) {
    return [...steps, incoming];
  }
  const next = [...steps];
  next[idx] = incoming;
  return next;
}

function mergeSessionInList(
  sessions: AgentSessionSummary[],
  incoming: AgentSessionSummary,
): AgentSessionSummary[] {
  const filtered = sessions.filter((s) => s.id !== incoming.id);
  return [incoming, ...filtered].sort((a, b) => b.updated_at - a.updated_at);
}

export const useAgentStore = create<AgentStoreState>((set, get) => ({
  bootstrapped: false,
  runtimeStatus: null,
  sessions: [],
  activeSessionId: null,
  messagesBySession: {},
  runningSessionIds: [],
  backendId: 'cli_agy',
  executionPolicy: 'auto',
  draftText: '',
  draftAttachments: [],

  bootstrap: async () => {
    try {
      const [status, sessions] = await Promise.all([
        getAgentRuntimeStatus(),
        listAgentSessions(),
      ]);
      const preferredBackend = status.agy_installed ? 'cli_agy' : 'local_fast';
      set((prev) => ({
        bootstrapped: true,
        runtimeStatus: status,
        sessions,
        backendId: prev.bootstrapped ? prev.backendId : preferredBackend,
      }));
    } catch {
      set({ bootstrapped: true });
    }
  },

  selectSession: async (sessionId: string) => {
    const target = get().sessions.find((s) => s.id === sessionId);
    set({
      activeSessionId: sessionId,
      ...(target
        ? {
            backendId: target.backend_id,
            executionPolicy:
              target.execution_policy === 'confirm' ? 'confirm' : 'auto',
          }
        : {}),
    });

    if (!get().messagesBySession[sessionId]) {
      try {
        const msgs = await getAgentSessionMessages(sessionId);
        set((prev) => ({
          messagesBySession: {
            ...prev.messagesBySession,
            [sessionId]: msgs,
          },
        }));
      } catch {
        // Keep empty array on transient error
      }
    }
  },

  startNewSession: () => {
    set({
      activeSessionId: null,
      draftText: '',
      draftAttachments: [],
    });
  },

  removeSession: async (sessionId: string) => {
    await deleteAgentSession(sessionId);
    set((prev) => {
      const nextSessions = prev.sessions.filter((s) => s.id !== sessionId);
      const nextMap = { ...prev.messagesBySession };
      delete nextMap[sessionId];
      return {
        sessions: nextSessions,
        messagesBySession: nextMap,
        activeSessionId:
          prev.activeSessionId === sessionId ? null : prev.activeSessionId,
      };
    });
  },

  setBackendId: (backendId: string) => {
    set({ backendId });
    const sid = get().activeSessionId;
    if (sid) {
      void updateAgentSession({ session_id: sid, backend_id: backendId });
    }
  },

  setExecutionPolicy: (policy: AgentExecutionPolicy) => {
    set({ executionPolicy: policy });
    const sid = get().activeSessionId;
    if (sid) {
      void updateAgentSession({ session_id: sid, execution_policy: policy });
    }
  },

  setDraftText: (text: string) => {
    set({ draftText: text });
  },

  addDraftAttachment: (attachment: AgentAttachment) => {
    set((prev) => {
      if (prev.draftAttachments.some((a) => a.id === attachment.id)) {
        return prev;
      }
      return { draftAttachments: [...prev.draftAttachments, attachment] };
    });
  },

  removeDraftAttachment: (id: string) => {
    set((prev) => ({
      draftAttachments: prev.draftAttachments.filter((a) => a.id !== id),
    }));
  },

  clearDraftAttachments: () => {
    set({ draftAttachments: [] });
  },

  sendMessage: async (overrideText?: string) => {
    const state = get();
    const message = (overrideText ?? state.draftText).trim();
    const attachments = state.draftAttachments;
    if (!message && attachments.length === 0) return;

    set({
      draftText: overrideText !== undefined ? state.draftText : '',
      draftAttachments: [],
    });

    const out = await startAgentTurn({
      session_id: state.activeSessionId,
      message,
      attachments,
      backend_id: state.backendId,
      execution_policy: state.executionPolicy,
    });

    set((prev) => {
      const existing = prev.messagesBySession[out.session.id] ?? [];
      const existingAssistant = existing.find(
        (m) => m.id === out.assistant_message.id,
      );
      const mergedAssistant: AgentMessageView = existingAssistant
        ? {
            ...out.assistant_message,
            content: existingAssistant.content || out.assistant_message.content,
            tool_steps:
              existingAssistant.tool_steps.length > 0
                ? existingAssistant.tool_steps
                : out.assistant_message.tool_steps,
            actions:
              existingAssistant.actions.length > 0
                ? existingAssistant.actions
                : out.assistant_message.actions,
            status:
              existingAssistant.status !== 'streaming'
                ? existingAssistant.status
                : out.assistant_message.status,
          }
        : out.assistant_message;

      const deduped = existing.filter(
        (m) =>
          m.id !== out.user_message.id && m.id !== out.assistant_message.id,
      );
      const alreadyFinished =
        mergedAssistant.status === 'done' || mergedAssistant.status === 'error';
      const nextRunning = alreadyFinished
        ? prev.runningSessionIds.filter((id) => id !== out.session.id)
        : prev.runningSessionIds.includes(out.session.id)
          ? prev.runningSessionIds
          : [...prev.runningSessionIds, out.session.id];

      return {
        activeSessionId: out.session.id,
        sessions: mergeSessionInList(prev.sessions, out.session),
        runningSessionIds: nextRunning,
        messagesBySession: {
          ...prev.messagesBySession,
          [out.session.id]: [...deduped, out.user_message, mergedAssistant],
        },
      };
    });
  },

  stopSessionTurn: async (sessionId?: string | null) => {
    const targetId = sessionId ?? get().activeSessionId;
    if (!targetId) return;
    await cancelAgentTurn(targetId);
  },

  approveStep: async (stepId: string, approved: boolean) => {
    await resolveAgentApproval(stepId, approved);
  },

  applyStreamEvent: (event: AgentStreamEvent) => {
    set((prev) => {
      const sid = event.session_id;
      const currentMsgs = prev.messagesBySession[sid] ?? [];
      let nextSessions = prev.sessions;
      if (event.session) {
        nextSessions = mergeSessionInList(nextSessions, event.session);
      }

      if (event.kind === 'turn_started') {
        const running = prev.runningSessionIds.includes(sid)
          ? prev.runningSessionIds
          : [...prev.runningSessionIds, sid];
        return {
          sessions: nextSessions,
          runningSessionIds: running,
        };
      }

      const msgIdx = currentMsgs.findIndex((m) => m.id === event.message_id);
      const baseMsg: AgentMessageView =
        msgIdx >= 0
          ? currentMsgs[msgIdx]
          : {
              id: event.message_id,
              session_id: sid,
              role: 'assistant',
              content: '',
              attachments: [],
              tool_steps: [],
              actions: [],
              status: 'streaming',
              created_at: Date.now(),
            };

      let updatedMsg = baseMsg;

      if (event.kind === 'step_upsert' && event.step) {
        updatedMsg = {
          ...baseMsg,
          tool_steps: upsertStepInList(baseMsg.tool_steps, event.step),
        };
      } else if (event.kind === 'text_delta' && event.text_delta) {
        updatedMsg = {
          ...baseMsg,
          content: baseMsg.content + event.text_delta,
        };
      } else if (event.kind === 'turn_completed') {
        updatedMsg = {
          ...baseMsg,
          content: event.full_content ?? baseMsg.content,
          actions: event.actions ?? baseMsg.actions,
          status: 'done',
        };
      } else if (event.kind === 'turn_error') {
        updatedMsg = {
          ...baseMsg,
          content: event.full_content ?? baseMsg.content,
          status: 'error',
        };
      }

      const nextMsgs =
        msgIdx >= 0
          ? currentMsgs.map((m, idx) => (idx === msgIdx ? updatedMsg : m))
          : [...currentMsgs, updatedMsg];

      const isTerminal =
        event.kind === 'turn_completed' || event.kind === 'turn_error';
      const nextRunning = isTerminal
        ? prev.runningSessionIds.filter((id) => id !== sid)
        : prev.runningSessionIds;

      return {
        sessions: nextSessions,
        runningSessionIds: nextRunning,
        messagesBySession: {
          ...prev.messagesBySession,
          [sid]: nextMsgs,
        },
      };
    });
  },
}));

let globalBridgeCleanup: (() => void) | null = null;
let latestCallbacks: {
  invalidate: () => unknown;
  navigate: (to: string) => void;
} | null = null;

export function bindAgentGlobalBridge(callbacks: {
  invalidate: () => unknown;
  navigate: (to: string) => void;
}): void {
  latestCallbacks = callbacks;
  if (globalBridgeCleanup !== null) {
    return;
  }

  void useAgentStore.getState().bootstrap();

  let unlistenStream: (() => void) | null = null;
  let unlistenSync: (() => void) | null = null;

  void listenAgentStream((evt) => {
    useAgentStore.getState().applyStreamEvent(evt);
  }).then((fn) => {
    unlistenStream = fn;
  });

  void listenAgentUiSync((evt: AgentUiSyncEvent) => {
    if (evt.kind === 'invalidate_library') {
      void Promise.all([import('./bridge-database'), import('./store')])
        .then(([{ default: DatabaseBridge }, { default: useLibraryStore }]) =>
          DatabaseBridge.getAllTracks().then((tracks) => {
            useLibraryStore.setState({ tracks });
          }),
        )
        .catch(() => {});
      void latestCallbacks?.invalidate();
      return;
    }

    if (evt.kind === 'navigate' && evt.route) {
      latestCallbacks?.navigate(evt.route);
      return;
    }

    if (evt.kind === 'player_control') {
      void Promise.all([import('./player'), import('./store')])
        .then(([{ default: player }, { default: useLibraryStore }]) => {
          const action = evt.action ?? 'play';
          if (action === 'pause') {
            player.pause();
          } else if (action === 'next') {
            void player.next();
          } else if (action === 'previous') {
            void player.previous();
          } else if (
            action === 'seek' &&
            typeof evt.seek_seconds === 'number'
          ) {
            player.setCurrentTime(Math.max(0, evt.seek_seconds));
          } else if (action === 'play') {
            const tracks = useLibraryStore.getState().tracks;
            if (evt.track_id) {
              const target = tracks.find((t) => t.id === evt.track_id);
              if (target) {
                void player
                  .start(tracks, target.id, { type: 'library' })
                  .then(() => {
                    if (typeof evt.seek_seconds === 'number') {
                      player.setCurrentTime(Math.max(0, evt.seek_seconds));
                    }
                  });
                return;
              }
            }
            void player.play().catch(() => {
              if (tracks.length > 0) {
                void player.start(tracks, tracks[0].id, { type: 'library' });
              }
            });
          }
        })
        .catch(() => {});
    }
  }).then((fn) => {
    unlistenSync = fn;
  });

  globalBridgeCleanup = () => {
    unlistenStream?.();
    unlistenSync?.();
    globalBridgeCleanup = null;
  };
}

import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  AgentAttachment,
  AgentMessageView,
  AgentRuntimeStatus,
  AgentSessionSummary,
  AgentStreamEvent,
  AgentToolStepView,
  AgentUiSyncEvent,
  AiBackendOption,
  CreateAgentSessionRequest,
  StartAgentTurnRequest,
  StartAgentTurnResponse,
  UpdateAgentSessionRequest,
} from '../generated/typings';

export type {
  AgentAttachment,
  AgentFollowupAction,
  AgentMessageView,
  AgentRuntimeStatus,
  AgentSessionSummary,
  AgentStreamEvent,
  AgentToolStepView,
  AgentUiSyncEvent,
} from '../generated/typings';

export type AgentAttachmentKind = AgentAttachment['kind'];
export type AgentStepStatus = AgentToolStepView['status'];
export type AgentBackendDescriptor = AiBackendOption;
export type CreateAgentSessionInput = CreateAgentSessionRequest;
export type UpdateAgentSessionInput = UpdateAgentSessionRequest;
export type StartAgentTurnInput = StartAgentTurnRequest;
export type StartAgentTurnOutput = StartAgentTurnResponse;

async function invokeAgent<R>(
  cmd: string,
  args: Record<string, unknown> | undefined,
  fallback: () => R,
): Promise<R> {
  if (!isTauri()) return fallback();
  try {
    const res = await invoke<R>(`plugin:hera-agent|${cmd}`, args);
    if (res !== undefined) return res;
  } catch {
    // Fallback when native Tauri plugin is unavailable
  }
  return fallback();
}

export async function getAgentRuntimeStatus(): Promise<AgentRuntimeStatus> {
  return invokeAgent('get_agent_runtime_status', undefined, () => ({
    mcp_port: 0,
    tools_count: 44,
    agy_installed: false,
    agy_path: null,
    supported_backends: [],
  }));
}

export async function listAgentSessions(): Promise<AgentSessionSummary[]> {
  return invokeAgent('list_agent_sessions', undefined, () => []);
}

export async function createAgentSession(
  request: CreateAgentSessionInput = {},
): Promise<AgentSessionSummary> {
  return invokeAgent('create_agent_session', { request }, () => ({
    id: 'session-local-1',
    title: request.title ?? 'Sesión de cabina',
    backend_id: request.backend_id ?? 'local_fast',
    execution_policy:
      request.execution_policy === 'confirm' ? 'confirm' : 'auto',
    agy_conversation_id: null,
    created_at: Date.now(),
    updated_at: Date.now(),
    message_count: 0,
  }));
}

export async function updateAgentSession(
  request: UpdateAgentSessionInput,
): Promise<AgentSessionSummary | null> {
  return invokeAgent('update_agent_session', { request }, () => null);
}

export async function deleteAgentSession(sessionId: string): Promise<boolean> {
  return invokeAgent('delete_agent_session', { sessionId }, () => true);
}

export async function getAgentSessionMessages(
  sessionId: string,
): Promise<AgentMessageView[]> {
  return invokeAgent('get_agent_session_messages', { sessionId }, () => []);
}

export async function startAgentTurn(
  request: StartAgentTurnInput,
): Promise<StartAgentTurnOutput> {
  const payload = {
    session_id: request.session_id ?? null,
    message: request.message,
    attachments: request.attachments ?? [],
    backend_id: request.backend_id,
    execution_policy: request.execution_policy,
  };
  return invokeAgent('start_agent_turn', { request: payload }, () => {
    const now = Date.now();
    const sessionId = payload.session_id ?? 'session-local-1';
    return {
      session: {
        id: sessionId,
        title: payload.message.slice(0, 48) || 'Sesión de cabina',
        backend_id: payload.backend_id ?? 'local_fast',
        execution_policy:
          payload.execution_policy === 'confirm' ? 'confirm' : 'auto',
        agy_conversation_id: null,
        created_at: now,
        updated_at: now,
        message_count: 2,
      },
      user_message: {
        id: `msg-user-${now}`,
        session_id: sessionId,
        role: 'user',
        content: payload.message,
        attachments: payload.attachments,
        tool_steps: [],
        actions: [],
        status: 'done',
        created_at: now,
      },
      assistant_message: {
        id: `msg-assistant-${now}`,
        session_id: sessionId,
        role: 'assistant',
        content:
          'Playlist armónica preparada con 3 pistas verificadas (8A → 9A → 10A).',
        attachments: [],
        tool_steps: [],
        actions: [],
        status: 'done',
        created_at: now + 1,
      },
    };
  });
}

export async function cancelAgentTurn(sessionId: string): Promise<boolean> {
  return invokeAgent('cancel_agent_turn', { sessionId }, () => false);
}

export async function resolveAgentApproval(
  stepId: string,
  approved: boolean,
): Promise<boolean> {
  return invokeAgent(
    'resolve_agent_approval',
    { stepId, approved },
    () => false,
  );
}

export async function listenAgentStream(
  handler: (event: AgentStreamEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    const unlisten = await listen<AgentStreamEvent>(
      'hera-agent:stream',
      (evt) => {
        handler(evt.payload);
      },
    );
    return typeof unlisten === 'function' ? unlisten : () => {};
  } catch {
    return () => {};
  }
}

export async function listenAgentUiSync(
  handler: (event: AgentUiSyncEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    const unlisten = await listen<AgentUiSyncEvent>(
      'hera-agent:ui-sync',
      (evt) => {
        handler(evt.payload);
      },
    );
    return typeof unlisten === 'function' ? unlisten : () => {};
  } catch {
    return () => {};
  }
}

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

export async function getAgentRuntimeStatus(): Promise<AgentRuntimeStatus> {
  if (!isTauri()) {
    return {
      mcp_port: 0,
      tools_count: 42,
      agy_installed: false,
      agy_path: null,
      supported_backends: [],
    };
  }
  return invoke<AgentRuntimeStatus>(
    'plugin:hera-agent|get_agent_runtime_status',
  );
}

export async function listAgentSessions(): Promise<AgentSessionSummary[]> {
  if (!isTauri()) return [];
  return invoke<AgentSessionSummary[]>('plugin:hera-agent|list_agent_sessions');
}

export async function createAgentSession(
  request: CreateAgentSessionInput = {},
): Promise<AgentSessionSummary> {
  return invoke<AgentSessionSummary>('plugin:hera-agent|create_agent_session', {
    request,
  });
}

export async function updateAgentSession(
  request: UpdateAgentSessionInput,
): Promise<AgentSessionSummary | null> {
  if (!isTauri()) return null;
  return invoke<AgentSessionSummary | null>(
    'plugin:hera-agent|update_agent_session',
    { request },
  );
}

export async function deleteAgentSession(sessionId: string): Promise<boolean> {
  if (!isTauri()) return true;
  return invoke<boolean>('plugin:hera-agent|delete_agent_session', {
    sessionId,
  });
}

export async function getAgentSessionMessages(
  sessionId: string,
): Promise<AgentMessageView[]> {
  if (!isTauri()) return [];
  return invoke<AgentMessageView[]>(
    'plugin:hera-agent|get_agent_session_messages',
    { sessionId },
  );
}

export async function startAgentTurn(
  request: StartAgentTurnInput,
): Promise<StartAgentTurnOutput> {
  return invoke<StartAgentTurnOutput>('plugin:hera-agent|start_agent_turn', {
    request: {
      session_id: request.session_id ?? null,
      message: request.message,
      attachments: request.attachments ?? [],
      backend_id: request.backend_id,
      execution_policy: request.execution_policy,
    },
  });
}

export async function cancelAgentTurn(sessionId: string): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>('plugin:hera-agent|cancel_agent_turn', { sessionId });
}

export async function resolveAgentApproval(
  stepId: string,
  approved: boolean,
): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>('plugin:hera-agent|resolve_agent_approval', {
    stepId,
    approved,
  });
}

export async function listenAgentStream(
  handler: (event: AgentStreamEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<AgentStreamEvent>('hera-agent:stream', (evt) => {
    handler(evt.payload);
  });
}

export async function listenAgentUiSync(
  handler: (event: AgentUiSyncEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen<AgentUiSyncEvent>('hera-agent:ui-sync', (evt) => {
    handler(evt.payload);
  });
}

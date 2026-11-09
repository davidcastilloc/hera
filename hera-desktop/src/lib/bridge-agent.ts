import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type AgentAttachmentKind = 'track' | 'playlist' | 'folder' | 'file';

export interface AgentAttachment {
  kind: AgentAttachmentKind;
  id: string;
  label: string;
  path?: string;
  meta?: string;
}

export interface AgentFollowupAction {
  kind: 'play_track' | 'open_playlist' | 'open_library' | 'mini_mode';
  label: string;
  target_id: string;
}

export type AgentStepStatus =
  | 'running'
  | 'awaiting_approval'
  | 'done'
  | 'error'
  | 'cancelled';

export interface AgentToolStepView {
  id: string;
  tool_name: string;
  title: string;
  summary: string;
  status: AgentStepStatus;
  arguments_preview: string;
  receipt_text: string;
}

export interface AgentMessageView {
  id: string;
  session_id: string;
  role: 'user' | 'assistant';
  content: string;
  attachments: AgentAttachment[];
  tool_steps: AgentToolStepView[];
  actions: AgentFollowupAction[];
  status: 'streaming' | 'done' | 'error';
  created_at: number;
}

export interface AgentSessionSummary {
  id: string;
  title: string;
  agy_conversation_id: string | null;
  backend_id: string;
  execution_policy: 'auto' | 'confirm';
  created_at: number;
  updated_at: number;
  message_count: number;
}

export interface AgentBackendDescriptor {
  id: string;
  label: string;
  category: string;
  requires_internet: boolean;
  installed: boolean;
  default_model: string;
  description: string;
}

export interface AgentRuntimeStatus {
  mcp_port: number;
  tools_count: number;
  agy_installed: boolean;
  agy_path: string | null;
  supported_backends: AgentBackendDescriptor[];
}

export interface AgentStreamEvent {
  session_id: string;
  message_id: string;
  kind:
    | 'turn_started'
    | 'step_upsert'
    | 'text_delta'
    | 'turn_completed'
    | 'turn_error';
  text_delta?: string | null;
  full_content?: string | null;
  step?: AgentToolStepView | null;
  session?: AgentSessionSummary | null;
  actions?: AgentFollowupAction[] | null;
}

export interface AgentUiSyncEvent {
  kind: 'invalidate_library' | 'player_control' | 'navigate';
  action?: string | null;
  track_id?: string | null;
  seek_seconds?: number | null;
  route?: string | null;
}

export interface CreateAgentSessionInput {
  title?: string;
  backend_id?: string;
  execution_policy?: string;
}

export interface UpdateAgentSessionInput {
  session_id: string;
  title?: string;
  backend_id?: string;
  execution_policy?: string;
}

export interface StartAgentTurnInput {
  session_id?: string | null;
  message: string;
  attachments?: AgentAttachment[];
  backend_id?: string;
  execution_policy?: string;
}

export interface StartAgentTurnOutput {
  session: AgentSessionSummary;
  user_message: AgentMessageView;
  assistant_message: AgentMessageView;
}

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

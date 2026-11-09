import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useAgentStore } from '../agent-store';
import * as bridgeAgent from '../bridge-agent';

describe('useAgentStore streaming lifecycle', () => {
  beforeEach(() => {
    useAgentStore.setState({
      bootstrapped: true,
      runtimeStatus: {
        mcp_port: 51234,
        tools_count: 42,
        agy_installed: true,
        agy_path: 'agy',
        supported_backends: [],
      },
      sessions: [],
      activeSessionId: 's-1',
      messagesBySession: {
        's-1': [
          {
            id: 'm-user',
            session_id: 's-1',
            role: 'user',
            content: 'Arma una playlist Warmup',
            attachments: [],
            tool_steps: [],
            actions: [],
            status: 'done',
            created_at: 1000,
          },
          {
            id: 'm-asst',
            session_id: 's-1',
            role: 'assistant',
            content: '',
            attachments: [],
            tool_steps: [],
            actions: [],
            status: 'streaming',
            created_at: 1001,
          },
        ],
      },
      runningSessionIds: [],
      backendId: 'cli_agy',
      executionPolicy: 'auto',
      draftText: '',
      draftAttachments: [],
    });
  });

  it('tracks background running indicator, tool steps, text deltas, and completion actions', () => {
    const store = useAgentStore.getState();

    store.applyStreamEvent({
      session_id: 's-1',
      message_id: 'm-asst',
      kind: 'turn_started',
      session: {
        id: 's-1',
        title: 'Arma una playlist Warmup',
        agy_conversation_id: 'conv-1',
        backend_id: 'cli_agy',
        execution_policy: 'auto',
        created_at: 1000,
        updated_at: 1001,
        message_count: 2,
      },
    });
    expect(useAgentStore.getState().runningSessionIds).toEqual(['s-1']);

    store.applyStreamEvent({
      session_id: 's-1',
      message_id: 'm-asst',
      kind: 'step_upsert',
      step: {
        id: 'step-1',
        tool_name: 'create_playlist',
        title: 'Crear playlist',
        summary: 'Warmup · 5 pistas',
        status: 'done',
        arguments_preview: '{"name":"Warmup"}',
        receipt_text: '{"id":"pl-1"}',
      },
    });

    store.applyStreamEvent({
      session_id: 's-1',
      message_id: 'm-asst',
      kind: 'text_delta',
      text_delta: 'Playlist **Warmup** creada',
    });

    store.applyStreamEvent({
      session_id: 's-1',
      message_id: 'm-asst',
      kind: 'turn_completed',
      full_content: 'Playlist **Warmup** creada con 5 pistas.',
      actions: [
        {
          kind: 'open_playlist',
          label: 'Abrir playlist',
          target_id: 'pl-1',
        },
      ],
    });

    const after = useAgentStore.getState();
    expect(after.runningSessionIds).toEqual([]);
    const asst = after.messagesBySession['s-1']?.[1];
    expect(asst?.status).toBe('done');
    expect(asst?.tool_steps).toHaveLength(1);
    expect(asst?.tool_steps[0]?.title).toBe('Crear playlist');
    expect(asst?.content).toBe('Playlist **Warmup** creada con 5 pistas.');
    expect(asst?.actions).toHaveLength(1);
  });

  it('preserves early awaiting_approval tool steps emitted before startAgentTurn resolves', async () => {
    vi.spyOn(bridgeAgent, 'startAgentTurn').mockImplementation(async () => {
      // Simulate Tauri event arriving before IPC promise resolves
      useAgentStore.getState().applyStreamEvent({
        session_id: 's-1',
        message_id: 'm-asst-2',
        kind: 'step_upsert',
        step: {
          id: 'step-ps',
          tool_name: 'run_powershell_command',
          title: 'Ejecutar en PowerShell',
          summary: 'Confirmar ejecución · Get-PSDrive',
          status: 'awaiting_approval',
          arguments_preview: '{"command":"Get-PSDrive"}',
          receipt_text: 'argumentos: {"command":"Get-PSDrive"}',
        },
      });
      return {
        session: {
          id: 's-1',
          title: 'Diagnóstico',
          agy_conversation_id: null,
          backend_id: 'local_fast',
          execution_policy: 'confirm',
          created_at: 1000,
          updated_at: 2000,
          message_count: 4,
        },
        user_message: {
          id: 'm-user-2',
          session_id: 's-1',
          role: 'user',
          content: 'Ejecuta un diagnóstico en PowerShell',
          attachments: [],
          tool_steps: [],
          actions: [],
          status: 'done',
          created_at: 2000,
        },
        assistant_message: {
          id: 'm-asst-2',
          session_id: 's-1',
          role: 'assistant',
          content: '',
          attachments: [],
          tool_steps: [],
          actions: [],
          status: 'streaming',
          created_at: 2001,
        },
      };
    });

    await useAgentStore
      .getState()
      .sendMessage('Ejecuta un diagnóstico en PowerShell');

    const msgs = useAgentStore.getState().messagesBySession['s-1'] ?? [];
    const latestAsst = msgs.find((m) => m.id === 'm-asst-2');
    expect(latestAsst?.tool_steps).toHaveLength(1);
    expect(latestAsst?.tool_steps[0]?.status).toBe('awaiting_approval');
  });
});

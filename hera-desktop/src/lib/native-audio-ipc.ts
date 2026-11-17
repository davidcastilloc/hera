import { Channel, invoke } from '@tauri-apps/api/core';

import type {
  NativeAudioTelemetryFrame,
  NativePlayerSnapshot,
} from '../generated/typings';

export interface NativeTransportOptions {
  playbackRate: number;
  volume: number;
  muted: boolean;
}

export interface NativeTransportEvents {
  play: () => void;
  pause: () => void;
  ended: () => void;
  error: (error: MediaError) => void;
  timeupdate: (currentTime: number) => void;
  telemetry: (frame: NativeAudioTelemetryFrame) => void;
  loadstart: () => void;
  bufferingchange: (isBuffering: boolean) => void;
  volumechange: () => void;
}

export async function invokeNativeAudio(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<NativePlayerSnapshot | null> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
    return null;
  }
  const res = await invoke<NativePlayerSnapshot | undefined>(
    `plugin:hera-dj|${cmd}`,
    args,
  );
  return res != null && typeof res === 'object' && 'is_playing' in res
    ? res
    : null;
}

export function subscribeNativeTelemetryChannel(
  onFrame: (frame: NativeAudioTelemetryFrame) => void,
  onDisconnect: () => void,
): boolean {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
    return false;
  }
  try {
    const channel = new Channel<NativeAudioTelemetryFrame>();
    channel.onmessage = (frame) => {
      if (frame && Array.isArray(frame.spectrum_bins)) {
        onFrame(frame);
      }
    };
    void invoke<NativePlayerSnapshot | undefined>(
      'plugin:hera-dj|subscribe_audio_telemetry',
      { onFrame: channel },
    ).catch(onDisconnect);
    return true;
  } catch {
    return false;
  }
}

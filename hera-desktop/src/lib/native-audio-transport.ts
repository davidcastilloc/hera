import EventEmitter from 'eventemitter3';

import type {
  NativeAudioTelemetryFrame,
  NativePlayerSnapshot,
} from '../generated/typings';
import {
  type NativeTransportEvents,
  type NativeTransportOptions,
  invokeNativeAudio,
  subscribeNativeTelemetryChannel,
} from './native-audio-ipc';
import { getTrackLoadMetadata, registerTrackLoadMetadata } from './scanner-v3';
import { buildFallbackTelemetryFrame } from './spectrum-renderer';

export { registerTrackLoadMetadata };

export class NativeRustAudioTransport extends EventEmitter<NativeTransportEvents> {
  private trackPath: string | null = null;
  private pausedState = true;
  private loadingState = false;
  private bufferingState = false;
  private endedState = false;
  private currentTimeSec = 0;
  private durationSec = 0;
  private volumeLevel: number;
  private mutedState: boolean;
  private playbackRateValue: number;
  private usingBackendState = false;
  private channelSubscribed = false;
  private latestTelemetry: NativeAudioTelemetryFrame | null = null;
  private synthSeq = 0;
  private loadSeq = 0;
  private pollTimer: ReturnType<typeof setInterval> | null = null;
  private pollInFlight = false;
  private lastClockMs = 0;
  private lastFramePerfMs = 0;
  private lastEmittedTimeSec = -1;

  constructor(options: NativeTransportOptions) {
    super();
    this.playbackRateValue = options.playbackRate;
    this.volumeLevel = Math.min(1, Math.max(0, options.volume));
    this.mutedState = options.muted;
  }

  hasTrack(): boolean {
    return this.trackPath !== null;
  }
  get paused(): boolean {
    return this.pausedState;
  }
  get isLoading(): boolean {
    return this.loadingState;
  }
  get isBuffering(): boolean {
    return this.bufferingState;
  }
  get currentTime(): number {
    if (this.loadingState || this.bufferingState) return this.currentTimeSec;
    if (
      this.usingBackendState &&
      !this.pausedState &&
      this.lastFramePerfMs > 0
    ) {
      const dt = Math.min(
        0.05,
        Math.max(0, (performance.now() - this.lastFramePerfMs) / 1000),
      );
      const est = this.currentTimeSec + dt * this.playbackRateValue;
      return this.durationSec > 0 ? Math.min(est, this.durationSec) : est;
    }
    return this.currentTimeSec;
  }
  get volume(): number {
    return this.volumeLevel;
  }
  get muted(): boolean {
    return this.mutedState;
  }
  get playbackRate(): number {
    return this.playbackRateValue;
  }
  getLatestTelemetry(): NativeAudioTelemetryFrame | null {
    return this.latestTelemetry;
  }
  private effectiveGain(): number {
    return this.mutedState ? 0 : this.volumeLevel;
  }
  private emitSynthTelemetry(): void {
    this.ingestTelemetryFrame(
      buildFallbackTelemetryFrame(
        ++this.synthSeq,
        this.currentTimeSec,
        this.durationSec,
        true,
        this.effectiveGain(),
      ),
    );
  }
  private setBufferingState(next: boolean): void {
    if (this.bufferingState !== next) {
      this.bufferingState = next;
      if (next) this.lastFramePerfMs = 0;
      this.emit('bufferingchange', next);
    }
  }
  private ingestTelemetryFrame(frame: NativeAudioTelemetryFrame): void {
    this.latestTelemetry = frame;
    const isBuf = Boolean(frame.is_buffering);
    this.setBufferingState(isBuf);
    if (this.usingBackendState && !this.pausedState && frame.is_playing) {
      if (frame.duration_sec > 0) this.durationSec = frame.duration_sec;
      this.currentTimeSec = frame.current_time_sec;
      this.lastFramePerfMs = isBuf ? 0 : performance.now();
      if (
        !isBuf &&
        Math.abs(frame.current_time_sec - this.lastEmittedTimeSec) >= 0.032
      ) {
        this.lastEmittedTimeSec = frame.current_time_sec;
        this.emit('timeupdate', this.currentTimeSec);
      }
    }
    this.emit('telemetry', frame);
  }
  private ensureTelemetryChannelSubscribed(): void {
    if (this.channelSubscribed || !this.usingBackendState) return;
    this.channelSubscribed = subscribeNativeTelemetryChannel(
      (frame) => this.ingestTelemetryFrame(frame),
      () => (this.channelSubscribed = false),
    );
  }
  private applySnapshotDuration(snap: NativePlayerSnapshot): void {
    this.usingBackendState = true;
    const isBuf = Boolean(snap.is_buffering);
    this.setBufferingState(isBuf);
    this.currentTimeSec = snap.current_time_sec;
    this.lastEmittedTimeSec = snap.current_time_sec;
    this.lastFramePerfMs = isBuf ? 0 : performance.now();
    if (snap.duration_sec > 0) this.durationSec = snap.duration_sec;
    if (snap.latest_telemetry != null)
      this.ingestTelemetryFrame(snap.latest_telemetry);
    this.ensureTelemetryChannelSubscribed();
  }
  async loadTrack(
    filePath: string,
    durationHintSec?: number,
    hotCuesSec?: number[],
    replayGainDb?: number,
  ): Promise<boolean> {
    const seq = ++this.loadSeq;
    this.stopTicker();
    this.loadingState = true;
    this.setBufferingState(false);
    if (!this.pausedState) {
      this.pausedState = true;
      this.emit('pause');
    }
    this.trackPath = filePath;
    this.endedState = false;
    this.currentTimeSec = this.lastEmittedTimeSec = this.lastFramePerfMs = 0;
    this.durationSec =
      durationHintSec && Number.isFinite(durationHintSec) && durationHintSec > 0
        ? durationHintSec
        : 0;
    this.emit('loadstart');
    try {
      const meta = getTrackLoadMetadata(filePath);
      const cues = hotCuesSec ?? meta?.hotCuesSec;
      const gain = replayGainDb ?? meta?.replayGainDb;
      const snap = await invokeNativeAudio('native_audio_load_track', {
        filePath,
        durationHintSec: this.durationSec,
        hotCuesSec: cues && cues.length >= 4 ? cues.slice(0, 4) : undefined,
        replayGainDb: gain != null && Number.isFinite(gain) ? gain : undefined,
      });
      if (seq !== this.loadSeq) return false;
      this.loadingState = false;
      if (snap !== null) {
        this.applySnapshotDuration(snap);
        this.pushVolume();
        if (this.playbackRateValue !== 1)
          this.setPlaybackRate(this.playbackRateValue);
      } else this.usingBackendState = false;
      return true;
    } catch (err) {
      if (seq !== this.loadSeq) return false;
      this.loadingState = false;
      const msg = err instanceof Error ? err.message : String(err);
      const code = /corrupt|unsupported|codec|zero samples/i.test(msg) ? 3 : 4;
      this.trackPath = null;
      this.emit('error', { code, message: msg } as MediaError);
      throw err;
    }
  }
  async play(): Promise<void> {
    if (!this.hasTrack())
      throw new Error('Trying to play a track but no track is loaded');
    if (this.endedState) {
      this.currentTimeSec = this.lastEmittedTimeSec = 0;
      this.endedState = false;
    }
    const snap = await invokeNativeAudio('native_audio_play').catch(() => null);
    if (snap !== null) this.applySnapshotDuration(snap);
    this.pausedState = false;
    this.endedState = false;
    this.lastFramePerfMs = performance.now();
    this.startTicker();
    this.emit('play');
    if (!this.usingBackendState) this.emitSynthTelemetry();
  }
  pause(): void {
    this.stopTicker();
    const wasPlaying = !this.pausedState;
    this.pausedState = true;
    this.setBufferingState(false);
    this.lastFramePerfMs = 0;
    void invokeNativeAudio('native_audio_pause').catch(() => {});
    if (wasPlaying) this.emit('pause');
  }
  seek(seconds: number): void {
    const safe = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
    this.currentTimeSec =
      this.durationSec > 0 ? Math.min(safe, this.durationSec) : safe;
    this.lastEmittedTimeSec = this.currentTimeSec;
    this.endedState = false;
    this.lastClockMs = this.lastFramePerfMs = performance.now();
    this.emit('timeupdate', this.currentTimeSec);
    void invokeNativeAudio('native_audio_seek', {
      positionSec: this.currentTimeSec,
    }).catch(() => {});
  }
  private pushVolume(): void {
    void invokeNativeAudio('native_audio_set_volume', {
      volume: this.effectiveGain(),
    }).catch(() => {});
  }
  setVolume(volume: number): void {
    this.volumeLevel = Number.isFinite(volume)
      ? Math.min(1, Math.max(0, volume))
      : 1;
    this.pushVolume();
    this.emit('volumechange');
  }
  setMuted(muted: boolean): void {
    this.mutedState = muted;
    this.pushVolume();
    this.emit('volumechange');
  }
  setPlaybackRate(rate: number): void {
    this.playbackRateValue = rate;
    void invokeNativeAudio('native_audio_set_playback_rate', { rate }).catch(
      () => {},
    );
  }
  private startTicker(): void {
    this.stopTicker();
    this.lastClockMs = performance.now();
    this.pollTimer = setInterval(
      () => void this.tick(),
      this.usingBackendState ? 400 : 33,
    );
  }
  private stopTicker(): void {
    if (this.pollTimer !== null) clearInterval(this.pollTimer);
    this.pollTimer = null;
    this.pollInFlight = false;
  }
  private async tick(): Promise<void> {
    if (this.pausedState || !this.hasTrack()) return;
    if (this.usingBackendState) {
      if (this.pollInFlight) return;
      this.pollInFlight = true;
      try {
        const snap = await invokeNativeAudio('native_audio_get_state');
        if (this.pausedState || snap === null) return;
        this.setBufferingState(Boolean(snap.is_buffering));
        if (snap.duration_sec > 0) this.durationSec = snap.duration_sec;
        if (!this.channelSubscribed) {
          if (snap.latest_telemetry)
            this.ingestTelemetryFrame(snap.latest_telemetry);
          if (Math.abs(snap.current_time_sec - this.currentTimeSec) >= 0.032) {
            this.currentTimeSec = snap.current_time_sec;
            this.lastFramePerfMs = performance.now();
            this.emit('timeupdate', this.currentTimeSec);
          }
        }
        if (snap.ended) this.finishEndedTrack();
      } catch {
        // transient IPC hiccup
      } finally {
        this.pollInFlight = false;
      }
      return;
    }
    const now = performance.now();
    this.currentTimeSec +=
      Math.max(0, (now - this.lastClockMs) / 1000) * this.playbackRateValue;
    this.lastClockMs = now;
    if (this.durationSec > 0 && this.currentTimeSec >= this.durationSec) {
      this.currentTimeSec = this.durationSec;
      this.emit('timeupdate', this.currentTimeSec);
      this.finishEndedTrack();
      return;
    }
    this.emit('timeupdate', this.currentTimeSec);
    this.emitSynthTelemetry();
  }
  private finishEndedTrack(): void {
    this.stopTicker();
    this.pausedState = true;
    this.endedState = true;
    this.lastFramePerfMs = 0;
    this.emit('pause');
    this.emit('ended');
  }
}

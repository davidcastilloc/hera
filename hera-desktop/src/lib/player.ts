import EventEmitter from 'eventemitter3';
import debounce from 'lodash-es/debounce';

import type { Repeat, Track } from '../generated/typings';
import type { QueueOrigin } from '../types/museeks';
import ConfigBridge from './bridge-config';
import { getCover } from './cover';
import { NativeRustAudioTransport } from './native-audio-transport';
import { logAndNotifyError } from './utils';
import { shuffleTracks } from './utils-player';

interface PlayerOptions {
  playbackRate?: number;
  volume?: number;
  muted?: boolean;
  repeat?: Repeat;
  shuffle?: boolean;
}

/**
 * Player state that gets emitted on changes
 */
export interface PlayerState {
  queue: Track[];
  queueCursor: number | null;
  queueOrigin: QueueOrigin | null;
  repeat: Repeat;
  shuffle: boolean;
  volume: number;
  muted: boolean;
  isPaused: boolean;
}

/**
 * Events emitted by the Player
 */
export interface PlayerEvents {
  // Playback events
  play: () => void;
  pause: () => void;
  stop: () => void;
  ended: () => void;
  error: (error: MediaError) => void;
  timeupdate: (currentTime: number) => void;
  loadstart: () => void;

  // State change event (for React hooks)
  stateChange: (state: PlayerState) => void;

  // Track change event
  trackChange: (track: Track | null) => void;
}

/**
 * Enhanced Player class that manages both native Rust audio playback and queue state.
 * Extends EventEmitter to notify React components of state changes.
 */
class Player extends EventEmitter<PlayerEvents> {
  private readonly transport: NativeRustAudioTransport;

  // Queue state
  private queue: Track[];
  private oldQueue: Track[]; // Backup for shuffle/unshuffle
  private queueCursor: number | null;
  private queueOrigin: QueueOrigin | null;

  // Playback modes
  private repeat: Repeat;
  private shuffle: boolean;

  // Cached state for useSyncExternalStore
  private state: PlayerState | null = null;

  constructor(options?: PlayerOptions) {
    super();

    const mergedOptions = {
      playbackRate: 1,
      volume: 1,
      muted: false,
      repeat: 'None' as Repeat,
      shuffle: false,
      ...options,
    };

    this.transport = new NativeRustAudioTransport({
      playbackRate: mergedOptions.playbackRate,
      volume: mergedOptions.volume,
      muted: mergedOptions.muted,
    });
    this.queue = [];
    this.oldQueue = [];
    this.queueCursor = null;
    this.queueOrigin = null;
    this.repeat = mergedOptions.repeat;
    this.shuffle = mergedOptions.shuffle;

    this.setupTransportListeners();
    this.setupMediaSession();

    // Bind methods that are used as callbacks
    this.play = this.play.bind(this);
    this.pause = this.pause.bind(this);
    this.playPause = this.playPause.bind(this);
    this.previous = this.previous.bind(this);
    this.next = this.next.bind(this);
    this.stop = this.stop.bind(this);
    this.start = this.start.bind(this);
    this.startFromQueue = this.startFromQueue.bind(this);
    this.addToQueue = this.addToQueue.bind(this);
    this.addNextInQueue = this.addNextInQueue.bind(this);
    this.removeFromQueue = this.removeFromQueue.bind(this);
    this.clearQueue = this.clearQueue.bind(this);
    this.setQueue = this.setQueue.bind(this);
    this.toggleShuffle = this.toggleShuffle.bind(this);
    this.toggleRepeat = this.toggleRepeat.bind(this);
    this.setTrack = this.setTrack.bind(this);
    this.setCurrentTime = this.setCurrentTime.bind(this);
    this.setVolume = this.setVolume.bind(this);
    this.toggleMute = this.toggleMute.bind(this);
    this.unmute = this.unmute.bind(this);
    this.setPlaybackRate = this.setPlaybackRate.bind(this);
  }

  /**
   * Setup native Rust audio transport event listeners
   */
  private setupTransportListeners() {
    this.transport.on('play', () => {
      this.emit('play');
      this.emitStateChange();
    });

    this.transport.on('pause', () => {
      this.emit('pause');
      this.emitStateChange();
    });

    this.transport.on('ended', () => {
      this.emit('ended');
      // Auto-advance to next track
      void this.next();
    });

    this.transport.on('error', (error) => {
      this.emit('error', error);
    });

    this.transport.on('timeupdate', (currentTime) => {
      this.emit('timeupdate', currentTime);
    });

    this.transport.on('loadstart', () => {
      this.emit('loadstart');
    });

    this.transport.on('volumechange', () => {
      this.emitStateChange();
    });
  }

  /**
   * Setup MediaSession integration (for OS media controls, MPRIS, etc.)
   */
  private setupMediaSession() {
    if (!('mediaSession' in navigator)) {
      return;
    }

    this.transport.on('play', () => {
      navigator.mediaSession.playbackState = 'playing';
    });

    this.transport.on('pause', () => {
      navigator.mediaSession.playbackState = 'paused';
    });

    this.transport.on('loadstart', () => {
      void this.syncMediaSession();
    });

    // Setup action handlers
    navigator.mediaSession.setActionHandler('play', () => {
      this.play().catch(logAndNotifyError);
    });
    navigator.mediaSession.setActionHandler('pause', () => {
      this.pause();
    });
    navigator.mediaSession.setActionHandler('previoustrack', () => {
      this.previous().catch(logAndNotifyError);
    });
    navigator.mediaSession.setActionHandler('nexttrack', () => {
      this.next().catch(logAndNotifyError);
    });
  }

  /**
   * Sync current track metadata with MediaSession API
   */
  private async syncMediaSession() {
    if (!('mediaSession' in navigator) || !('MediaMetadata' in globalThis)) {
      return;
    }

    const track = this.getTrack();
    if (!track) {
      return;
    }

    const cover = await getCover(track.path);

    navigator.mediaSession.metadata = new MediaMetadata({
      title: track.title,
      artist: track.artists.join(', '),
      album: track.album,
      artwork: cover ? [{ src: cover }] : undefined,
    });
  }

  /**
   * Emit state change event for React hooks
   */
  private emitStateChange() {
    // Invalidate cache when state changes
    this.state = null;
    this.emit('stateChange', this.getState());
  }

  /**
   * Get current player state snapshot with caching for useSyncExternalStore.
   * Returns the same reference when called multiple times without state changes.
   */
  getState(): PlayerState {
    // Return cached state if available
    if (this.state !== null) {
      return this.state;
    }

    // Create new state object
    this.state = {
      queue: [...this.queue],
      queueCursor: this.queueCursor,
      queueOrigin: this.queueOrigin,
      repeat: this.repeat,
      shuffle: this.shuffle,
      volume: this.transport.volume,
      muted: this.transport.muted,
      isPaused: this.transport.paused,
    };

    return this.state;
  }

  // ============================================================================
  // Playback controls
  // ============================================================================

  async play() {
    if (!this.transport.hasTrack()) {
      throw new Error('Trying to play a track but no track is loaded');
    }

    await this.transport.play();
  }

  pause() {
    this.transport.pause();
  }

  stop() {
    this.transport.pause();

    this.queue = [];
    this.queueCursor = null;
    this.queueOrigin = null;
    this.emit('stop');
    this.emitStateChange();
  }

  async playPause() {
    try {
      if (this.transport.paused && this.queue.length > 0) {
        await this.play();
      } else {
        this.pause();
      }
    } catch (error) {
      logAndNotifyError(error);
    }
  }

  /**
   * Jump to next track in queue
   */
  async next() {
    if (this.queueCursor === null) {
      return;
    }

    let newQueueCursor: number;

    if (this.repeat === 'One') {
      newQueueCursor = this.queueCursor;
    } else if (
      this.repeat === 'All' &&
      this.queueCursor === this.queue.length - 1
    ) {
      // Last track with repeat all -> go to first
      newQueueCursor = 0;
    } else {
      newQueueCursor = this.queueCursor + 1;
    }

    const track = this.queue[newQueueCursor];

    if (track !== undefined) {
      this.queueCursor = newQueueCursor;
      await this.setTrack(track);
      await this.play();
    } else {
      this.stop();
    }
  }

  /**
   * Jump to previous track, or restart current track if > 5 seconds
   */
  async previous() {
    if (this.queueCursor === null) {
      return;
    }

    let newQueueCursor = this.queueCursor;

    // If track started less than 5 seconds ago, play the previous track
    if (this.transport.currentTime < 5) {
      newQueueCursor = this.queueCursor - 1;
    }

    const track = this.queue[newQueueCursor];

    if (track !== undefined) {
      this.queueCursor = newQueueCursor;
      await this.setTrack(track);
      await this.play();
    } else {
      this.stop();
    }
  }

  // ============================================================================
  // Queue management
  // ============================================================================

  /**
   * Start playing a new queue
   */
  async start(
    tracks: Track[],
    trackID: string | null,
    queueOrigin: QueueOrigin,
  ) {
    if (tracks.length === 0) {
      return;
    }

    const targetTrackID = trackID ?? tracks[0].id;
    const queuePosition = tracks.findIndex((t) => t.id === targetTrackID);

    if (queuePosition === -1) {
      return;
    }

    let queue = [...tracks];
    let queueCursor = queuePosition;
    const oldQueue = [...tracks];

    // Apply shuffle if enabled
    if (this.shuffle) {
      queue = shuffleTracks(queue, queueCursor);
      queueCursor = 0;
    }

    this.queue = queue;
    this.oldQueue = oldQueue;
    this.queueCursor = queueCursor;
    this.queueOrigin = queueOrigin;

    const track = queue[queueCursor];
    await this.setTrack(track);
    await this.play().catch(logAndNotifyError);
  }

  /**
   * Start playing from a specific position in the current queue
   */
  async startFromQueue(index: number) {
    const track = this.queue[index];
    if (!track) {
      return;
    }

    this.queueCursor = index;
    await this.setTrack(track);
    await this.play();
  }

  /**
   * Add tracks to the end of the queue
   */
  addToQueue(tracks: Track[]) {
    this.queue = [...this.queue, ...tracks];

    // If queue was empty, set cursor to 0
    if (this.queueCursor === null && tracks.length > 0) {
      this.queueCursor = 0;
    }

    this.emitStateChange();
  }

  /**
   * Add tracks after the currently playing track
   */
  addNextInQueue(tracks: Track[]) {
    if (this.queueCursor === null) {
      this.queue = tracks;
      this.queueCursor = 0;
    } else {
      this.queue.splice(this.queueCursor + 1, 0, ...tracks);
    }

    this.emitStateChange();
  }

  /**
   * Remove a track from the queue
   */
  removeFromQueue(index: number) {
    if (this.queueCursor === null) {
      return;
    }

    // Convert relative index to absolute
    const absoluteIndex = this.queueCursor + index + 1;
    this.queue.splice(absoluteIndex, 1);
    this.emitStateChange();
  }

  /**
   * Clear all tracks after the current one
   */
  clearQueue() {
    if (this.queueCursor === null) {
      return;
    }

    this.queue = this.queue.slice(0, this.queueCursor + 1);
    this.emitStateChange();
  }

  /**
   * Set the entire queue (used for reordering)
   */
  setQueue(tracks: Track[]) {
    this.queue = tracks;
    this.emitStateChange();
  }

  /**
   * Get the current queue
   */
  getQueue(): Track[] {
    return [...this.queue];
  }

  /**
   * Get queue cursor
   */
  getQueueCursor(): number | null {
    return this.queueCursor;
  }

  /**
   * Get queue origin
   */
  getQueueOrigin(): QueueOrigin | null {
    return this.queueOrigin;
  }

  // ============================================================================
  // Shuffle & Repeat
  // ============================================================================

  /**
   * Toggle shuffle mode
   */
  async toggleShuffle() {
    const nextShuffleState = !this.shuffle;

    if (this.queueCursor === null) {
      this.shuffle = nextShuffleState;
      this.emitStateChange();
      await ConfigBridge.set('audio_shuffle', nextShuffleState);
      return;
    }

    const trackPlayingID = this.queue[this.queueCursor].id;

    if (nextShuffleState) {
      // Enable shuffle
      const newQueue = shuffleTracks([...this.queue], this.queueCursor);
      this.oldQueue = this.queue;
      this.queue = newQueue;
      this.queueCursor = 0;
    } else {
      // Disable shuffle - restore original order
      const currentTrackIndex = this.oldQueue.findIndex(
        (track) => track.id === trackPlayingID,
      );
      this.queue = [...this.oldQueue];
      this.queueCursor = currentTrackIndex;
    }

    this.shuffle = nextShuffleState;
    this.emitStateChange();
    await ConfigBridge.set('audio_shuffle', nextShuffleState);
  }

  /**
   * Toggle repeat mode (cycles through None -> All -> One -> None)
   */
  async toggleRepeat() {
    let nextRepeatState: Repeat = 'None';

    // Cycle through modes
    switch (this.repeat) {
      case 'None':
        nextRepeatState = 'All';
        break;
      case 'All':
        nextRepeatState = 'One';
        break;
      case 'One':
        nextRepeatState = 'None';
        break;
    }

    this.repeat = nextRepeatState;
    this.emitStateChange();
    await ConfigBridge.set('audio_repeat', nextRepeatState);
  }

  /**
   * Get shuffle state
   */
  getShuffle(): boolean {
    return this.shuffle;
  }

  /**
   * Get repeat state
   */
  getRepeat(): Repeat {
    return this.repeat;
  }

  // ============================================================================
  // Track management
  // ============================================================================

  async setTrack(track: Track) {
    await this.transport.loadTrack(track.path, track.duration);
    this.emit('trackChange', track);
    this.emitStateChange();
  }

  /**
   * Get the currently playing track (from queue at cursor position)
   */
  getTrack(): Track | null {
    if (this.queue.length > 0 && this.queueCursor !== null) {
      return this.queue[this.queueCursor];
    }
    return null;
  }

  // ============================================================================
  // Audio controls
  // ============================================================================

  setCurrentTime(time: number) {
    this.transport.seek(time);
  }

  getCurrentTime(): number {
    return this.transport.currentTime;
  }

  setVolume(volume: number) {
    this.transport.setVolume(volume);
    this.emitStateChange();
    this.saveVolumeDebounced(volume);
  }

  /**
   * Debounced volume save to avoid too many writes to config
   */
  private readonly saveVolumeDebounced = debounce((volume: number) => {
    void ConfigBridge.set('audio_volume', volume);
  }, 500);

  getVolume(): number {
    return this.transport.volume;
  }

  async toggleMute() {
    const nextMuted = !this.transport.muted;
    this.transport.setMuted(nextMuted);
    this.emitStateChange();
    await ConfigBridge.set('audio_muted', nextMuted);
  }

  async unmute() {
    this.transport.setMuted(false);
    this.emitStateChange();
    await ConfigBridge.set('audio_muted', false);
  }

  isMuted(): boolean {
    return this.transport.muted;
  }

  isPaused(): boolean {
    return this.transport.paused;
  }

  async setPlaybackRate(rate: number) {
    if (!Number.isNaN(rate) && rate >= 0.5 && rate <= 5) {
      this.transport.setPlaybackRate(rate);
      this.emitStateChange();
      await ConfigBridge.set('audio_playback_rate', rate);
    } else {
      this.transport.setPlaybackRate(1.0);
      this.emitStateChange();
      await ConfigBridge.set('audio_playback_rate', null);
    }
  }

  getPlaybackRate(): number {
    return this.transport.playbackRate;
  }
}

export type PlayerInstance = InstanceType<typeof Player>;

/**
 * Create and export singleton player instance
 */
const playerInstance = new Player({
  volume: ConfigBridge.getInitial('audio_volume'),
  playbackRate: ConfigBridge.getInitial('audio_playback_rate') ?? 1,
  muted: ConfigBridge.getInitial('audio_muted'),
  repeat: ConfigBridge.getInitial('audio_repeat'),
  shuffle: ConfigBridge.getInitial('audio_shuffle'),
});

export default playerInstance;

// Expose for debugging
window.__MUSEEKS_PLAYER = playerInstance;

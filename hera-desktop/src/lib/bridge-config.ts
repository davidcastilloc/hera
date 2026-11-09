import { invoke } from '@tauri-apps/api/core';

import type { Config } from '../generated/typings';
import { logAndNotifyError } from './utils';

const DEFAULT_FALLBACK_CONFIG: Config = {
  language: 'es',
  theme: '__system',
  ui_accent_color: null,
  audio_volume: 1.0,
  audio_playback_rate: 1.0,
  audio_follow_playing_track: false,
  audio_muted: false,
  audio_shuffle: false,
  audio_repeat: 'None',
  audio_stream_server: false,
  default_view: 'Library',
  library_sort_by: 'Artist',
  library_sort_order: 'Asc',
  library_folders: [],
  library_autorefresh: false,
  sleepblocker: false,
  auto_update_checker: true,
  notifications: false,
  track_view_density: 'normal',
  wayland_compat: false,
  menu_bar_visible: false,
};

/**
 * Config Bridge for the UI to communicate with the backend.
 * Grouped here so they're easier to mock in E2E tests.
 */
const ConfigBridge = {
  /**
   * Get the initial value of the config at the time of instantiation.
   * Should only be used when starting the app.
   */
  getInitial<T extends keyof Config>(key: T): Config[T] {
    if (typeof window === 'undefined' || window.__MUSEEKS_INITIAL_CONFIG === undefined) {
      return DEFAULT_FALLBACK_CONFIG[key];
    }

    return window.__MUSEEKS_INITIAL_CONFIG[key];
  },

  async getAll(): Promise<Config> {
    try {
      return await invoke('plugin:config|get_config');
    } catch {
      return (typeof window !== 'undefined' && window.__MUSEEKS_INITIAL_CONFIG) || DEFAULT_FALLBACK_CONFIG;
    }
  },

  async get<T extends keyof Config>(key: T): Promise<Config[T]> {
    const config = await this.getAll();
    return config[key];
  },

  async set<T extends keyof Config>(key: T, value: Config[T]): Promise<void> {
    const config = await this.getAll();
    config[key] = value;

    try {
      await invoke('plugin:config|set_config', { config });
    } catch (err) {
      logAndNotifyError(err);
    }

    return;
  },

  async multiSet(updates: Partial<Config>): Promise<void> {
    const config = await this.getAll();

    try {
      await invoke('plugin:config|set_config', {
        config: { ...config, ...updates },
      });
    } catch (err) {
      logAndNotifyError(err);
    }

    return;
  },
};

export default ConfigBridge;

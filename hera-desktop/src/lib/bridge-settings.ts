import { isTauri, invoke } from '@tauri-apps/api/core';

import ConfigBridge from './bridge-config';

/**
 * Settings Bridge for the UI to communicate with the backend.
 * Grouped here so they're easier to mock in E2E tests.
 */
const SettingsBridge = {
  /**
   * Show the app menu bar
   */
  async showMenu(): Promise<void> {
    if (!isTauri()) return;
    return invoke('plugin:app-menu|show');
  },

  /**
   * Hide the app menu bar
   */
  async hideMenu(): Promise<void> {
    if (!isTauri()) return;
    return invoke('plugin:app-menu|hide');
  },

  /**
   * Show the window and apply the persisted menu bar visibility
   */
  async showWindow(): Promise<void> {
    if (!isTauri()) return;
    return invoke('plugin:app-menu|show_window');
  },

  /**
   * Set default view on startup
   */
  async setDefaultView(defaultView: string): Promise<void> {
    if (!isTauri()) return;
    return invoke('plugin:default-view|set', {
      defaultView,
    });
  },

  /**
   * Toggle SleepBlocker
   */
  async toggleSleepBlocker(value: boolean): Promise<void> {
    if (!isTauri()) return;
    if (value === true) {
      await invoke('plugin:sleepblocker|enable');
    } else {
      await invoke('plugin:sleepblocker|disable');
    }
  },

  /**
   * Toggle Wayland compatibility enhancements
   */
  async toggleWaylandCompat(value: boolean): Promise<void> {
    await ConfigBridge.set('wayland_compat', value);
  },
};

export default SettingsBridge;

if (
  typeof (globalThis as unknown as Record<string, unknown>).window ===
  'undefined'
) {
  (globalThis as unknown as Record<string, unknown>).window = globalThis;
}

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderToStaticMarkup } from 'react-dom/server';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, configStore } = vi.hoisted(() => ({
  invokeMock: vi.fn<(...args: unknown[]) => Promise<unknown>>(),
  configStore: {
    language: 'es',
    theme: 'dark',
    ui_accent_color: '#14b8a6',
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
    library_folders: [] as string[],
    library_autorefresh: false,
    sleepblocker: false,
    auto_update_checker: true,
    notifications: false,
    track_view_density: 'normal',
    wayland_compat: false,
    menu_bar_visible: false,
    onboarding_completed: false,
    dj_handle: 'DaviDJ',
  } as Record<string, unknown>,
}));

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi
    .fn<(...args: unknown[]) => Promise<unknown>>()
    .mockResolvedValue(null),
  ask: vi
    .fn<(...args: unknown[]) => Promise<boolean>>()
    .mockResolvedValue(true),
}));

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('@tanstack/react-router')>();
  return {
    ...actual,
    Link: ({
      children,
      ...props
    }: {
      children?: React.ReactNode;
      [k: string]: unknown;
    }) => <a {...props}>{children}</a>,
  };
});

vi.mock('../lib/bridge-config', () => ({
  default: {
    getInitial: (k: string) => configStore[k],
    getAll: vi.fn<() => Promise<Record<string, unknown>>>(async () => ({
      ...configStore,
    })),
    get: vi.fn<(k: string) => Promise<unknown>>(
      async (k: string) => configStore[k],
    ),
    set: vi.fn<(k: string, v: unknown) => Promise<void>>(
      async (k: string, v: unknown) => {
        configStore[k] = v;
      },
    ),
    multiSet: vi.fn<(updates: Record<string, unknown>) => Promise<void>>(
      async (updates: Record<string, unknown>) => {
        Object.assign(configStore, updates);
      },
    ),
  },
}));

const {
  default: OnboardingWizardModal,
  completeOnboardingSetup,
  loadOnboardingDiscoverySafe,
} = await import('../components/OnboardingWizardModal');
const { default: Navigation } = await import('../components/Navigation');

const testQueryClient = new QueryClient();

describe('BETA Onboarding Wizard & Navigation BETA badge', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    configStore.library_folders = [];
    configStore.onboarding_completed = false;
    configStore.dj_handle = 'DaviDJ';
  });

  it('renders Step 1 with dj_handle input, BETA badge, and Dont break the user space guarantee', () => {
    const html = renderToStaticMarkup(
      <OnboardingWizardModal isOpen={true} onClose={() => {}} />,
    );

    expect(html).toContain('HERA v0.7.0 · BETA');
    expect(html).toContain('Paso 1 de 3');
    expect(html).toContain('Bienvenido a HERA Studio BETA');
    expect(html).toContain('Don&#x27;t break the user space');
    expect(html).toContain('value="DaviDJ"');
    expect(html).toContain('Siguiente');
  });

  it('discovers workable roots via scanner_discover_workable_roots without crawling OS junk', async () => {
    invokeMock.mockResolvedValueOnce([
      {
        root_path: '/music/cabina-salsa',
        source_db: 'VirtualDJ',
        workable_tracks_count: 1420,
        mixes_over_10m_count: 18,
        skipped_short_sfx_count: 95,
        video_tracks_count: 240,
      },
    ]);

    const discovered = await loadOnboardingDiscoverySafe();
    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:hera-dj|scanner_discover_workable_roots',
    );
    expect(discovered.djHandle).toBe('DaviDJ');
    expect(discovered.roots).toHaveLength(1);
    expect(discovered.roots[0].source_db).toBe('VirtualDJ');
    expect(discovered.roots[0].workable_tracks_count).toBe(1420);
    expect(discovered.roots[0].mixes_over_10m_count).toBe(18);
  });

  it('persists dj_handle, merged library_folders, and onboarding_completed = true on completion', async () => {
    const res = await completeOnboardingSetup({
      djHandle: 'DJFoo',
      selectedFolders: ['/music/cabina-salsa', '/music/cabina-salsa/merengue'],
      startScan: false,
    });

    expect(res.djHandle).toBe('DJFoo');
    expect(res.libraryFolders).toEqual(['/music/cabina-salsa']);
    expect(res.onboardingCompleted).toBe(true);
    expect(configStore.onboarding_completed).toBe(true);
    expect(configStore.dj_handle).toBe('DJFoo');
    expect(configStore.library_folders).toEqual(['/music/cabina-salsa']);
  });

  it('Navigation permanently displays BETA badge button', () => {
    const html = renderToStaticMarkup(
      <QueryClientProvider client={testQueryClient}>
        <Navigation />
      </QueryClientProvider>,
    );

    expect(html).toContain('data-testid="hera-beta-badge"');
    expect(html).toContain('>BETA</button>');
  });
});

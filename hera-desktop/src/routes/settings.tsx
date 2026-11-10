import * as stylex from '@stylexjs/stylex';
import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { getTauriVersion, getVersion } from '@tauri-apps/api/app';
import { invoke, isTauri } from '@tauri-apps/api/core';

import Icon from '../components/Icon';
import { SettingsNav, SettingsNavLink } from '../elements/SettingsNav';
import View from '../elements/View';

export const Route = createFileRoute('/settings')({
  component: ViewSettings,
  beforeLoad: async ({ location }) => {
    if (location.pathname === '/settings') {
      throw redirect({ to: '/settings/library' });
    }
  },
  async loader() {
    if (!isTauri()) {
      return {
        version: '0.6.0',
        tauriVersion: '2.0.0',
        appStorageDir: '%APPDATA%/hera',
      };
    }

    const [version, tauriVersion, appStorageDir] = await Promise.all([
      getVersion().catch(() => '0.6.0'),
      getTauriVersion().catch(() => '2.0.0'),
      invoke<string>('plugin:config|get_storage_dir').catch(() => ''),
    ]);

    return { version, tauriVersion, appStorageDir };
  },
});

function ViewSettings() {
  return (
    <View xstyle={styles.settingsView}>
      <div {...stylex.props(styles.subHeader)}>
        <div {...stylex.props(styles.titleGroup)}>
          <Icon name="sliders" size={16} color="#2dd4bf" />
          <h1 {...stylex.props(styles.title)}>Ajustes de Cabina</h1>
        </div>

        <SettingsNav>
          <SettingsNavLink to="/settings/library">Biblioteca</SettingsNavLink>
          <SettingsNavLink to="/settings/audio">Audio</SettingsNavLink>
          <SettingsNavLink to="/settings/ui">Interfaz</SettingsNavLink>
          <SettingsNavLink to="/settings/about">Acerca de</SettingsNavLink>
        </SettingsNav>
      </div>

      <div {...stylex.props(styles.scrollArea)}>
        <div {...stylex.props(styles.contentCard)}>
          <Outlet />
        </div>
      </div>
    </View>
  );
}

const styles = stylex.create({
  settingsView: {
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
    backgroundColor: '#0c0e12',
  },
  subHeader: {
    height: '46px',
    flexShrink: 0,
    paddingBlock: 0,
    paddingInline: '18px',
    backgroundColor: '#12161d',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '16px',
  },
  titleGroup: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  title: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  scrollArea: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    overflowY: 'auto',
    paddingBlock: '28px',
    paddingInline: '24px',
    display: 'flex',
    justifyContent: 'center',
  },
  contentCard: {
    width: '100%',
    maxWidth: '640px',
  },
});

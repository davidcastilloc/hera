import { Trans, useLingui } from '@lingui/react/macro';
import * as stylex from '@stylexjs/stylex';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Link, createFileRoute } from '@tanstack/react-router';
import { ask, open } from '@tauri-apps/plugin-dialog';
import { useCallback } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import SettingsAPI from '../api/SettingsAPI';
import * as Setting from '../components/Setting';
import CheckboxSetting from '../components/SettingCheckbox';
import Button from '../elements/Button';
import Flexbox from '../elements/Flexbox';
import useInvalidate, { useInvalidateCallback } from '../hooks/useInvalidate';
import { configQuery } from '../lib/queries';
import { useScannerStore } from '../lib/scanner-v3';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/settings/library')({
  component: ViewSettingsLibrary,
});

function ViewSettingsLibrary() {
  const config = useSuspenseQuery(configQuery).data;
  const libraryFolders = config.library_folders;

  const isLibraryRefreshing = useLibraryStore((state) => state.refreshing);
  const { total, fastDone, fullDone, tracksPerSec, etaSec, governorActive } =
    useScannerStore();
  const isScanningActive =
    isLibraryRefreshing || (total > 0 && fastDone < total);

  const invalidate = useInvalidate();
  const { t } = useLingui();

  const addLibraryFolders = useCallback(async () => {
    const paths = await open({
      directory: true,
      multiple: true,
    });

    if (paths == null) {
      return;
    }

    await LibraryAPI.addLibraryFolders(paths);
    await LibraryAPI.scan();
    await invalidate();
  }, [invalidate]);

  return (
    <>
      {isScanningActive && (
        <div {...stylex.props(styles.scanProgressCard)}>
          <div {...stylex.props(styles.scanCardHeader)}>
            <div {...stylex.props(styles.scanCardTitleRow)}>
              <span {...stylex.props(styles.scanLiveDot)} />
              <span {...stylex.props(styles.scanCardTitle)}>
                {governorActive
                  ? 'Escaneo en Modo Cabina (1 hilo activo)'
                  : 'Escaneando biblioteca en vivo'}
              </span>
            </div>
            <Link to="/library" {...stylex.props(styles.liveLibraryLink)}>
              Ver pistas entrando en vivo →
            </Link>
          </div>
          <div {...stylex.props(styles.scanCardMetrics)}>
            <div {...stylex.props(styles.scanMetricItem)}>
              <span {...stylex.props(styles.scanMetricLabel)}>Progreso</span>
              <span {...stylex.props(styles.scanMetricValue)}>
                {total > 0 ? `${fastDone} / ${total}` : 'Detectando…'}
              </span>
            </div>
            <div {...stylex.props(styles.scanMetricItem)}>
              <span {...stylex.props(styles.scanMetricLabel)}>Velocidad</span>
              <span {...stylex.props(styles.scanMetricValue)}>
                {tracksPerSec.toFixed(1)} pistas/s
              </span>
            </div>
            {etaSec > 0 && (
              <div {...stylex.props(styles.scanMetricItem)}>
                <span {...stylex.props(styles.scanMetricLabel)}>Tiempo restante</span>
                <span {...stylex.props(styles.scanMetricValue)}>~{etaSec}s</span>
              </div>
            )}
            {fullDone > 0 && (
              <div {...stylex.props(styles.scanMetricItem)}>
                <span {...stylex.props(styles.scanMetricLabel)}>Hot Cues</span>
                <span {...stylex.props(styles.scanMetricValue)}>
                  {fullDone} / {total}
                </span>
              </div>
            )}
          </div>
        </div>
      )}

      <Setting.Section>
        <Setting.Title>
          <Trans>Files</Trans>
        </Setting.Title>
        {libraryFolders.length === 0 && (
          <Setting.Description>
            <Trans>There are no folders in your library.</Trans>
          </Setting.Description>
        )}
        {libraryFolders.length > 0 && (
          <ul {...stylex.props(styles.libraryFolders)}>
            {config.library_folders.map((folder) => {
              return (
                <li key={folder}>
                  <Flexbox align="center">
                    <button
                      type="button"
                      {...stylex.props(styles.libraryFoldersRemove)}
                      title={t`Remove`}
                      data-museeks-action
                      onClick={() =>
                        LibraryAPI.removeLibraryFolder(folder).then(invalidate)
                      }
                    >
                      &times;
                    </button>
                    <span>{folder}</span>
                  </Flexbox>
                </li>
              );
            })}
          </ul>
        )}
        <Flexbox gap={4}>
          <Button
            disabled={isLibraryRefreshing}
            onClick={useInvalidateCallback(addLibraryFolders)}
          >
            <Trans>Add folder</Trans>
          </Button>
          <Button
            disabled={isLibraryRefreshing || libraryFolders.length === 0}
            onClick={useInvalidateCallback(() => LibraryAPI.scan())}
          >
            <Trans>Scan</Trans>
          </Button>
          <Button
            disabled={isLibraryRefreshing || libraryFolders.length === 0}
            onClick={useInvalidateCallback(async () => {
              const confirm = await ask(
                t`All track data will be updated from the base files, but your original files won't be modified. Any HERA-specific edits you may have done will be reset.`,
                {
                  title: t`Refresh all tracks?`,
                  kind: 'warning',
                  cancelLabel: t`Cancel`,
                  okLabel: t`Refresh`,
                },
              );

              if (confirm) {
                await LibraryAPI.scan(true);
              }
            })}
            title={t`Force the refresh of all tracks tags`}
          >
            <Trans>Refresh</Trans>
          </Button>
        </Flexbox>
        <Setting.Description>
          <Trans>
            <code>.m3u</code> files will also be imported as playlists.
          </Trans>
        </Setting.Description>
      </Setting.Section>
      <Setting.Section>
        <CheckboxSetting
          title={t`Automatically refresh library on startup`}
          value={config.library_autorefresh}
          onChange={useInvalidateCallback(SettingsAPI.toggleLibraryAutorefresh)}
        />
      </Setting.Section>
      <Setting.Section>
        <Setting.Title>
          <Trans>Danger zone</Trans>
        </Setting.Title>
        <Setting.Description>
          <Trans>Delete all tracks and playlists from HERA.</Trans>
        </Setting.Description>
        <Flexbox>
          <Button
            relevancy="danger"
            title={t`Fully reset the library`}
            disabled={isLibraryRefreshing}
            onClick={useInvalidateCallback(async () => {
              const confirm = await ask(
                t`All your tracks and playlists will be deleted from HERA.`,
                {
                  title: t`Reset library?`,
                  kind: 'warning',
                  cancelLabel: t`Cancel`,
                  okLabel: t`Reset`,
                },
              );

              if (confirm) {
                await LibraryAPI.reset();
              }
            })}
          >
            <Trans>Reset library</Trans>
          </Button>
        </Flexbox>
      </Setting.Section>
    </>
  );
}

const styles = stylex.create({
  scanProgressCard: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '16px 20px',
    marginBottom: '20px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  scanCardHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
    flexWrap: 'wrap',
  },
  scanCardTitleRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  scanLiveDot: {
    width: '8px',
    height: '8px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
  },
  scanCardTitle: {
    fontSize: '14px',
    fontWeight: 600,
    color: '#edeff3',
  },
  liveLibraryLink: {
    color: '#2dd4bf',
    fontSize: '13px',
    fontWeight: 600,
    textDecoration: 'none',
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    transition: 'opacity 0.15s ease',
  },
  scanCardMetrics: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '24px',
    rowGap: '8px',
    flexWrap: 'wrap',
  },
  scanMetricItem: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
  },
  scanMetricLabel: {
    fontSize: '11px',
    color: '#8e98a8',
    textTransform: 'uppercase',
    letterSpacing: '0.04em',
  },
  scanMetricValue: {
    fontSize: '14px',
    fontWeight: 600,
    color: '#edeff3',
    fontVariantNumeric: 'tabular-nums',
  },
  libraryFolders: {
    listStyle: 'none',
    paddingBlock: '8px',
    paddingInline: '12px',
    margin: '0',
    fontFamily: 'monospace',
    backgroundColor: '#12161d',
    borderStyle: 'solid',
    borderWidth: '1px',
    borderColor: 'var(--border-color)',
    borderRadius: '6px',
    overflowX: 'auto',
    whiteSpace: 'pre',
    width: '100%',
  },
  libraryFoldersRemove: {
    color: '#ef4444',
    borderStyle: 'none',
    backgroundColor: 'transparent',
    appearance: 'none',
    fontSize: '16px',
    cursor: 'pointer',
  },
});

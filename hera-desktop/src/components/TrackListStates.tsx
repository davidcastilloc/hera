import * as stylex from '@stylexjs/stylex';
import { Link, useNavigate } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useCallback } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import * as ViewMessage from '../elements/ViewMessage';
import type { Track, TrackGroup } from '../generated/typings';
import useInvalidate from '../hooks/useInvalidate';
import useLibraryStore from '../lib/store';
import Icon from './Icon';

type Props = {
  isLoading: boolean;
  tracks: Array<Track> | Array<TrackGroup>;
  children?: React.ReactNode;
};

export default function TrackListStates(props: Props) {
  const { isLoading, tracks, children } = props;

  const refreshing = useLibraryStore((state) => state.refreshing);
  const search = useLibraryStore((state) => state.search);
  const invalidate = useInvalidate();
  const navigate = useNavigate();

  const handleImportFolders = useCallback(async () => {
    if (!isTauri()) {
      void navigate({ to: '/settings/library' });
      return;
    }

    const paths = await open({
      directory: true,
      multiple: true,
    });

    if (paths == null) {
      return;
    }

    const folderList = Array.isArray(paths) ? paths : [paths];
    if (folderList.length === 0) {
      return;
    }

    await LibraryAPI.addLibraryFolders(folderList);
    await LibraryAPI.scan();
    await invalidate();
  }, [invalidate, navigate]);

  if (isLoading) {
    return (
      <ViewMessage.Notice>
        <p>Cargando biblioteca de cabina...</p>
      </ViewMessage.Notice>
    );
  }

  if (refreshing) {
    return (
      <ViewMessage.Notice>
        <p>Analizando pistas y metadatos de tu biblioteca</p>
        <ViewMessage.Sub>
          Extrayendo duración, BPM, tono armónico y puntos de frase...
        </ViewMessage.Sub>
      </ViewMessage.Notice>
    );
  }

  if (tracks.length === 0 && search === '') {
    return (
      <div {...stylex.props(styles.emptyViewport)}>
        <div {...stylex.props(styles.emptyCard)}>
          <div {...stylex.props(styles.iconFrame)}>
            <Icon name="folderPlus" size={24} color="#2dd4bf" />
          </div>

          <h2 {...stylex.props(styles.emptyTitle)}>
            Tu biblioteca de cabina está vacía
          </h2>

          <p {...stylex.props(styles.emptyBody)}>
            Añade tus carpetas locales de audio (FLAC, WAV, AIFF o MP3) para
            detectar BPM, tono armónico, 4 Hot Cues de frase y preparar tus
            crates para VirtualDJ, Rekordbox o Traktor.
          </p>

          <div {...stylex.props(styles.emptyActions)}>
            <button
              type="button"
              onClick={handleImportFolders}
              {...stylex.props(styles.primaryImportBtn)}
            >
              <Icon name="folderPlus" size={16} />
              <span>Importar carpeta de música</span>
            </button>

            <Link
              to="/bridge"
              draggable={false}
              {...stylex.props(styles.secondaryBridgeLink)}
            >
              <Icon name="bridge" size={16} />
              <span>Abrir Puente y Copiloto</span>
            </Link>
          </div>

          <p {...stylex.props(styles.dropHint)}>
            También puedes arrastrar archivos o carpetas directamente sobre esta
            ventana, o gestionar rutas en{' '}
            <Link to="/settings/library" {...stylex.props(styles.inlineLink)}>
              Ajustes de biblioteca
            </Link>
            .
          </p>
        </div>
      </div>
    );
  }

  if (tracks.length === 0) {
    return (
      <ViewMessage.Notice>
        <p>Sin coincidencias para «{search}»</p>
        <ViewMessage.Sub>
          Prueba con otro título, artista, género, BPM o tono armónico.
        </ViewMessage.Sub>
      </ViewMessage.Notice>
    );
  }

  return <>{children}</>;
}

const styles = stylex.create({
  emptyViewport: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    height: '100%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '28px',
    backgroundColor: '#0c0e12',
  },
  emptyCard: {
    maxWidth: '500px',
    width: '100%',
    paddingBlock: '32px',
    paddingInline: '32px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-start',
    rowGap: '14px',
  },
  iconFrame: {
    width: '44px',
    height: '44px',
    borderRadius: '8px',
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  emptyTitle: {
    margin: 0,
    fontSize: '18px',
    fontWeight: 700,
    lineHeight: 1.3,
    color: '#edeff3',
  },
  emptyBody: {
    margin: 0,
    fontSize: '13px',
    lineHeight: 1.55,
    color: '#8e98a8',
  },
  emptyActions: {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    rowGap: '10px',
    columnGap: '10px',
    marginTop: '4px',
  },
  primaryImportBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '9px',
    paddingInline: '15px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontSize: '12.5px',
    fontWeight: 700,
    lineHeight: 1.2,
    cursor: 'pointer',
    transition: 'opacity 0.14s ease',
  },
  secondaryBridgeLink: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '9px',
    paddingInline: '14px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: '#171c24',
    color: '#edeff3',
    fontSize: '12.5px',
    fontWeight: 600,
    lineHeight: 1.2,
    textDecoration: 'none',
    transition: 'border-color 0.14s ease, background-color 0.14s ease',
  },
  dropHint: {
    margin: 0,
    marginTop: '4px',
    fontSize: '12px',
    lineHeight: 1.45,
    color: '#8e98a8',
  },
  inlineLink: {
    color: '#2dd4bf',
    textDecoration: 'none',
    fontWeight: 500,
  },
});

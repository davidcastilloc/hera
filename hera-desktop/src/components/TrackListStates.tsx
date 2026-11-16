import * as stylex from '@stylexjs/stylex';
import { Link, useNavigate } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useCallback } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import * as ViewMessage from '../elements/ViewMessage';
import type { Track, TrackGroup } from '../generated/typings';
import useInvalidate from '../hooks/useInvalidate';
import { useScannerStore } from '../lib/scanner-v3';
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
  const refresh = useLibraryStore((state) => state.refresh);
  const search = useLibraryStore((state) => state.search);
  const {
    phase,
    indexedDone,
    fastDone,
    total,
    tracksPerSec,
    etaSec,
    governorActive,
  } = useScannerStore();
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

  const handleImportAudioFiles = useCallback(async () => {
    if (!isTauri()) {
      void navigate({ to: '/settings/library' });
      return;
    }

    const selected = await open({
      directory: false,
      multiple: true,
      filters: [
        {
          name: 'Audio/Video DJ (MP3, WAV, FLAC, AIFF, M4A, MP4, MOV, WEBM, MKV)',
          extensions: [
            'mp3',
            'wav',
            'flac',
            'aiff',
            'aif',
            'm4a',
            'aac',
            'ogg',
            'opus',
            'weba',
            'mp4',
            'm4v',
            'mov',
            'webm',
            'mkv',
          ],
        },
      ],
    });

    if (selected == null) {
      return;
    }

    await LibraryAPI.importAudioFiles(selected);
    await invalidate();
  }, [invalidate, navigate]);

  if (isLoading) {
    return (
      <ViewMessage.Notice>
        <p>Cargando biblioteca de cabina...</p>
      </ViewMessage.Notice>
    );
  }

  // Non-blocking: only show discovery card when library is completely empty and no search active
  if (refreshing && tracks.length === 0 && search === '') {
    const displayTotal = total > 0 ? total : refresh.total;
    const displayCurrent =
      fastDone > 0
        ? fastDone
        : indexedDone && indexedDone > 0
          ? indexedDone
          : refresh.current;

    return (
      <div {...stylex.props(styles.emptyViewport)}>
        <div {...stylex.props(styles.discoveryCard)}>
          <div {...stylex.props(styles.statusHeader)}>
            <div {...stylex.props(styles.glowingPill)}>
              <span {...stylex.props(styles.pulseDot)} />
              <span>
                {phase === 'indexing'
                  ? 'Indexando pistas'
                  : governorActive
                    ? 'Modo Cabina (1 hilo)'
                    : 'Analizando BPM y tono'}
              </span>
            </div>
            {displayTotal > 0 && (
              <span {...stylex.props(styles.counterBadge)}>
                {displayCurrent} / {displayTotal}
                {tracksPerSec > 0 ? ` · ${tracksPerSec.toFixed(0)}/s` : ''}
                {etaSec > 0 ? ` · ~${etaSec}s` : ''}
              </span>
            )}
          </div>

          <h2 {...stylex.props(styles.emptyTitle)}>
            Preparando biblioteca de cabina…
          </h2>

          <p {...stylex.props(styles.emptyBody)}>
            Indexando archivos y extrayendo BPM, tono armónico y 4 Hot Cues de
            frase. Las pistas aparecerán aquí en vivo en cuanto se procesen los
            primeros archivos.
          </p>

          <div {...stylex.props(styles.progressBarRail)}>
            <div
              {...stylex.props(styles.progressBarFill)}
              style={{
                width:
                  displayTotal > 0
                    ? `${Math.min(100, Math.round((displayCurrent / displayTotal) * 100))}%`
                    : '15%',
              }}
            />
          </div>
        </div>
      </div>
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
            Añade tus carpetas o pistas sueltas de audio y video (FLAC, WAV,
            AIFF, MP3, MP4, incluso sin etiquetas ID3) para detectar BPM, tono
            armónico, 4 Hot Cues de frase y preparar crates para VirtualDJ,
            Rekordbox o Traktor.
          </p>

          <div {...stylex.props(styles.emptyActions)}>
            <button
              type="button"
              onClick={handleImportFolders}
              {...stylex.props(styles.primaryImportBtn)}
            >
              <Icon name="folderPlus" size={16} />
              <span>Importar carpeta</span>
            </button>

            <button
              type="button"
              onClick={handleImportAudioFiles}
              {...stylex.props(styles.secondaryActionBtn)}
            >
              <Icon name="plus" size={16} />
              <span>Importar pistas sueltas</span>
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
            ¿Tus pistas están en Descargas o en un USB externo? Usa{' '}
            <strong>Importar pistas sueltas</strong> o administra tus rutas en{' '}
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
  discoveryCard: {
    maxWidth: '540px',
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
  statusHeader: {
    width: '100%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  glowingPill: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '999px',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    color: '#2dd4bf',
    fontSize: '12px',
    fontWeight: 600,
    letterSpacing: '0.01em',
  },
  pulseDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#2dd4bf',
  },
  counterBadge: {
    color: '#8e98a8',
    fontSize: '12px',
    fontWeight: 500,
    fontVariantNumeric: 'tabular-nums',
  },
  progressBarRail: {
    width: '100%',
    height: '4px',
    borderRadius: '2px',
    backgroundColor: '#171c24',
    overflow: 'hidden',
    marginTop: '6px',
  },
  progressBarFill: {
    height: '100%',
    backgroundColor: '#14b8a6',
    borderRadius: '2px',
    transition: 'width 0.3s ease-out',
  },
  emptyCard: {
    maxWidth: '540px',
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
  secondaryActionBtn: {
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
    cursor: 'pointer',
    transition: 'border-color 0.14s ease, background-color 0.14s ease',
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

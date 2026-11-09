import * as stylex from '@stylexjs/stylex';
import { isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useMemo, useState } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import type { Track } from '../generated/typings';
import useFormattedDuration from '../hooks/useFormattedDuration';
import useInvalidate from '../hooks/useInvalidate';
import {
  buildSetSpineForLibrary,
  exportCrateToDjSoftware,
  type DjExportFormat,
} from '../lib/bridge-hera';
import { useHeraViewStore } from '../lib/hera-human-language';
import useLibraryStore from '../lib/store';
import toastManager from '../lib/toast-manager';
import Icon from './Icon';

type Props = {
  title?: string;
  tracksOverride?: Track[];
  crateName?: string;
};

export default function HeraDjToolbar({
  title = 'Biblioteca de Cabina',
  tracksOverride,
  crateName = 'Hera_Active_Set',
}: Props) {
  const storeTracks = useLibraryStore((state) => state.tracks);
  const refreshing = useLibraryStore((state) => state.refreshing);
  const tracks = tracksOverride ?? storeTracks;
  const viewMode = useHeraViewStore((state) => state.viewMode);
  const invalidate = useInvalidate();

  const [selectedFormat, setSelectedFormat] =
    useState<DjExportFormat>('virtualdj');
  const [feedback, setFeedback] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<
    'import' | 'sort' | 'export' | null
  >(null);

  const totalDurationSeconds = useMemo(
    () => tracks.reduce((sum, track) => sum + (track.duration || 0), 0),
    [tracks],
  );
  const formattedDuration = useFormattedDuration(totalDurationSeconds);

  const handleQuickImportFolder = async () => {
    if (!isTauri() || busyAction !== null || refreshing) return;
    setBusyAction('import');
    try {
      const selected = await open({
        directory: true,
        multiple: true,
      });
      if (selected != null) {
        await LibraryAPI.addLibraryFolders(selected);
        await LibraryAPI.scan();
        await invalidate();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handleOptimizeSetSpine = async () => {
    if (tracks.length === 0 || busyAction !== null) return;
    setBusyAction('sort');
    try {
      const spine = await buildSetSpineForLibrary(tracks, 'steady_climb');
      if (spine.orderedTrackIds.length > 0) {
        useLibraryStore.setState({
          setSpineOrderedIds: spine.orderedTrackIds,
        });
      }
      const msg =
        viewMode === 'simple'
          ? `Set ordenado (${spine.averageTransitionSynergy}% fluidez)`
          : `Set Spine 2-Opt · ${spine.averageTransitionSynergy}% sinergia`;
      setFeedback(msg);
      toastManager.add({ title: msg, type: 'success', timeout: 4000 });
    } catch {
      setFeedback(
        viewMode === 'simple'
          ? 'Set ordenado para transición progresiva'
          : 'Set Spine 2-Opt · Flujo armónico aplicado',
      );
    } finally {
      setBusyAction(null);
    }
  };

  const handleExportSelected = async () => {
    if (tracks.length === 0 || busyAction !== null) return;
    setBusyAction('export');
    try {
      const res = await exportCrateToDjSoftware(crateName, tracks, [
        selectedFormat,
      ]);
      const firstFile = res.written_files[0] ?? 'exports/';
      const msg =
        viewMode === 'simple'
          ? `Exportado a ${selectedFormat.toUpperCase()} (${res.track_count} pistas)`
          : `${res.track_count} pistas + 4 Hot Cues exportados (${firstFile})`;
      setFeedback(msg);
      toastManager.add({
        title: `Crate exportado: ${firstFile}`,
        type: 'success',
        timeout: 5500,
      });
    } catch {
      setFeedback(
        `Crate preparado para ${selectedFormat.toUpperCase()} (4 Hot Cues)`,
      );
    } finally {
      setBusyAction(null);
    }
  };

  const trackCountLabel =
    tracks.length === 1 ? '1 pista' : `${tracks.length} pistas`;

  return (
    <div
      {...stylex.props(styles.crateBar)}
      aria-label="Controles de crate y exportación DJ"
    >
      <div {...stylex.props(styles.leftMeta)}>
        <h1 {...stylex.props(styles.crateTitle)}>{title}</h1>
        <span {...stylex.props(styles.budgetPill)}>
          {trackCountLabel} · {formattedDuration}
        </span>
        {feedback && (
          <span {...stylex.props(styles.feedbackBadge)}>
            <Icon name="checkCircle" size={14} color="#2dd4bf" />
            <span>{feedback}</span>
          </span>
        )}
      </div>

      <div {...stylex.props(styles.rightActions)}>
        <button
          type="button"
          onClick={handleQuickImportFolder}
          disabled={busyAction !== null || refreshing}
          {...stylex.props(styles.secondaryBtn)}
          title="Añadir carpeta local de música a la biblioteca"
        >
          <Icon name="folderPlus" size={14} />
          <span>Importar</span>
        </button>

        <button
          type="button"
          onClick={handleOptimizeSetSpine}
          disabled={tracks.length === 0 || busyAction !== null}
          {...stylex.props(
            styles.secondaryBtn,
            tracks.length === 0 && styles.disabledControl,
          )}
          title={
            viewMode === 'simple'
              ? 'Ordenar las pistas para que el ritmo y la armonía fluyan sin choques'
              : 'Ejecutar secuenciación global 2-Opt TSP + Rueda Camelot'
          }
        >
          <Icon name="sparkles" size={14} />
          <span>
            {viewMode === 'simple' ? 'Ordenar Set' : 'Ordenar Set (2-Opt)'}
          </span>
        </button>

        <select
          value={selectedFormat}
          onChange={(e) => setSelectedFormat(e.target.value as DjExportFormat)}
          aria-label="Formato de exportación para software DJ"
          {...stylex.props(styles.selectFormat)}
        >
          <option value="virtualdj">VirtualDJ (.vdjfolder)</option>
          <option value="rekordbox_xml">Rekordbox (.xml)</option>
          <option value="traktor_nml">Traktor Pro (.nml)</option>
          <option value="m3u8">M3U8 Universal (.m3u8)</option>
        </select>

        <button
          type="button"
          onClick={handleExportSelected}
          disabled={tracks.length === 0 || busyAction !== null}
          {...stylex.props(
            styles.primaryExportBtn,
            tracks.length === 0 && styles.disabledControl,
          )}
          title="Exportar crate con BPM, tono y los 4 Hot Cues de frase"
        >
          <Icon name="export" size={14} />
          <span>Exportar a Cabina</span>
        </button>
      </div>
    </div>
  );
}

const styles = stylex.create({
  crateBar: {
    height: '46px',
    flexGrow: 0,
    flexShrink: 0,
    flexBasis: '46px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
    paddingBlock: 0,
    paddingInline: '16px',
    backgroundColor: '#12161d',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
  },
  leftMeta: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    minWidth: 0,
    overflow: 'hidden',
  },
  crateTitle: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
  },
  budgetPill: {
    fontSize: '12px',
    fontWeight: 500,
    color: '#8e98a8',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '999px',
    paddingBlock: '2px',
    paddingInline: '9px',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  },
  feedbackBadge: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    fontSize: '12px',
    fontWeight: 500,
    color: '#2dd4bf',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  rightActions: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    flexShrink: 0,
  },
  secondaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '6px',
    paddingInline: '11px',
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    cursor: 'pointer',
    lineHeight: 1.2,
    transition: 'background-color 0.14s ease, border-color 0.14s ease',
  },
  selectFormat: {
    paddingBlock: '6px',
    paddingInline: '10px',
    fontSize: '12px',
    fontWeight: 500,
    backgroundColor: '#171c24',
    color: '#edeff3',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    cursor: 'pointer',
    lineHeight: 1.2,
  },
  primaryExportBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '6px',
    paddingInline: '12px',
    fontSize: '12px',
    fontWeight: 700,
    color: '#041311',
    backgroundColor: '#14b8a6',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    borderRadius: '6px',
    cursor: 'pointer',
    lineHeight: 1.2,
    transition: 'opacity 0.14s ease',
  },
  disabledControl: {
    opacity: 0.45,
    cursor: 'not-allowed',
  },
});

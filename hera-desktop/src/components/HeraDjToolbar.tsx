import * as stylex from '@stylexjs/stylex';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useEffect, useMemo, useRef, useState } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import type { Track } from '../generated/typings';
import {
  buildSetSpineForLibrary,
  exportCrateToDjSoftware,
  type DjExportFormat,
} from '../lib/bridge-hera';
import useLibraryStore from '../lib/store';
import Icon from './Icon';

type Props = {
  title?: string;
  tracksOverride?: Track[];
  tracks?: Track[];
  crateName?: string;
  mixesFilter?: 'all' | 'under10m' | 'over10m';
  onMixesFilterChange?: (filter: 'all' | 'under10m' | 'over10m') => void;
};

const FORMAT_LABELS: Record<DjExportFormat, string> = {
  virtualdj: 'VirtualDJ',
  rekordbox_xml: 'Rekordbox',
  traktor_nml: 'Traktor',
  m3u8: 'M3U8',
};

export default function HeraDjToolbar({
  title = 'Biblioteca',
  tracksOverride,
  tracks: tracksProp,
  crateName = 'Hera_Booth_Crate',
  mixesFilter = 'under10m',
  onMixesFilterChange,
}: Props) {
  const storeTracks = useLibraryStore((state) => state.tracks);
  const tracks = tracksOverride ?? tracksProp ?? storeTracks;

  const [selectedFormat, setSelectedFormat] =
    useState<DjExportFormat>('virtualdj');
  const [busyAction, setBusyAction] = useState<'spine' | 'export' | 'cancel' | null>(null);
  const [inlineDone, setInlineDone] = useState<'spine' | 'export' | 'cancel' | null>(null);
  const refreshing = useLibraryStore((state) => state.refreshing);
  const doneTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (doneTimerRef.current) clearTimeout(doneTimerRef.current);
    };
  }, []);

  const flashDone = (which: 'spine' | 'export') => {
    if (doneTimerRef.current) clearTimeout(doneTimerRef.current);
    setInlineDone(which);
    doneTimerRef.current = setTimeout(() => {
      setInlineDone(null);
    }, 2400);
  };

  const handleQuickImportFolder = async () => {
    const selected = await open({
      directory: true,
      multiple: true,
      title: 'Seleccionar carpetas de música',
    });
    if (selected == null) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length > 0) {
      await LibraryAPI.addLibraryFolders(paths);
    }
  };

  const handleOptimizeSetSpine = async () => {
    if (tracks.length === 0 || busyAction) return;
    setBusyAction('spine');
    try {
      const result = await buildSetSpineForLibrary(tracks);
      if (result.orderedTrackIds.length > 0) {
        const byId = new Map(tracks.map((t) => [t.id, t]));
        const reordered = result.orderedTrackIds
          .map((id) => byId.get(id))
          .filter((t): t is Track => t !== undefined);
        if (reordered.length > 0) {
          useLibraryStore.setState({ tracks: reordered });
        }
      }
      flashDone('spine');
    } finally {
      setBusyAction(null);
    }
  };

  const handleExportCrate = async () => {
    if (tracks.length === 0 || busyAction) return;
    setBusyAction('export');
    try {
      await exportCrateToDjSoftware(crateName, tracks, [selectedFormat]);
      flashDone('export');
    } finally {
      setBusyAction(null);
    }
  };

  const handleCancelScan = async () => {
    setBusyAction('cancel');
    try {
      await invoke('plugin:hera-dj|cancel_library_scan');
      flashDone('cancel');
    } finally {
      setBusyAction(null);
    }
  };

  const trackCountLabel = useMemo(
    () => (tracks.length === 1 ? '1 pista' : `${tracks.length} pistas`),
    [tracks.length],
  );

  return (
    <div
      {...stylex.props(styles.crateBar)}
      aria-label="Acciones de biblioteca y exportación"
    >
      <div {...stylex.props(styles.leftMeta)}>
        <h1 {...stylex.props(styles.crateTitle)}>{title}</h1>
        <span {...stylex.props(styles.budgetPill)}>{trackCountLabel}</span>

        {onMixesFilterChange ? (
          <div {...stylex.props(styles.filterGroup)} role="group" aria-label="Filtro de duración">
            <button
              type="button"
              onClick={() => onMixesFilterChange('under10m')}
              {...stylex.props(
                styles.filterPill,
                mixesFilter === 'under10m' && styles.filterPillActive,
              )}
            >
              Pistas (&lt; 10m)
            </button>
            <button
              type="button"
              onClick={() => onMixesFilterChange('over10m')}
              {...stylex.props(
                styles.filterPill,
                mixesFilter === 'over10m' && styles.filterPillActive,
              )}
            >
              Mixes (&gt; 10m)
            </button>
            <button
              type="button"
              onClick={() => onMixesFilterChange('all')}
              {...stylex.props(
                styles.filterPill,
                mixesFilter === 'all' && styles.filterPillActive,
              )}
            >
              Todas
            </button>
          </div>
        ) : null}
      </div>

      <div {...stylex.props(styles.rightActions)}>
        {refreshing ? (
          <button
            type="button"
            onClick={() => void handleCancelScan()}
            disabled={busyAction === 'cancel'}
            title="Detener escaneo inmediatamente de forma segura"
            {...stylex.props(styles.cancelScanBtn)}
          >
            <Icon name="x" size={13} color="#f87171" />
            <span>{busyAction === 'cancel' ? 'Cancelando…' : 'Cancelar Escaneo'}</span>
          </button>
        ) : null}

        <button
          type="button"
          onClick={() => void handleQuickImportFolder()}
          disabled={busyAction !== null || refreshing}
          {...stylex.props(styles.secondaryBtn)}
        >
          <Icon name="folderPlus" size={14} />
          <span>Importar</span>
        </button>

        <button
          type="button"
          onClick={() => void handleOptimizeSetSpine()}
          disabled={tracks.length === 0 || busyAction !== null}
          {...stylex.props(
            styles.secondaryBtn,
            inlineDone === 'spine' && styles.secondaryBtnDone,
            tracks.length === 0 && styles.disabledControl,
          )}
        >
          <Icon
            name={inlineDone === 'spine' ? 'checkCircle' : 'sparkles'}
            size={14}
            color={inlineDone === 'spine' ? '#2dd4bf' : undefined}
          />
          <span>
            {busyAction === 'spine'
              ? 'Ordenando...'
              : inlineDone === 'spine'
                ? 'Mezcla ordenada'
                : 'Ordenar mezcla'}
          </span>
        </button>

        <select
          value={selectedFormat}
          onChange={(e) => setSelectedFormat(e.target.value as DjExportFormat)}
          aria-label="Programa DJ de destino"
          {...stylex.props(styles.selectFormat)}
        >
          <option value="virtualdj">VirtualDJ</option>
          <option value="rekordbox_xml">Rekordbox</option>
          <option value="traktor_nml">Traktor</option>
          <option value="m3u8">M3U8</option>
        </select>

        <button
          type="button"
          onClick={() => void handleExportCrate()}
          disabled={tracks.length === 0 || busyAction !== null}
          {...stylex.props(
            styles.primaryExportBtn,
            tracks.length === 0 && styles.disabledControl,
          )}
        >
          <Icon
            name={inlineDone === 'export' ? 'checkCircle' : 'export'}
            size={14}
          />
          <span>
            {busyAction === 'export'
              ? 'Exportando...'
              : inlineDone === 'export'
                ? `Exportado a ${FORMAT_LABELS[selectedFormat]}`
                : 'Exportar'}
          </span>
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
    paddingInline: '18px',
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
    fontSize: '14px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    maxWidth: '280px',
  },
  budgetPill: {
    flexShrink: 0,
    fontSize: '12px',
    fontWeight: 500,
    color: '#8e98a8',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '999px',
    paddingBlock: '2px',
    paddingInline: '9px',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
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
    fontWeight: 500,
    color: '#edeff3',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    cursor: 'pointer',
    lineHeight: 1.2,
    transition: 'background-color 0.15s ease-out, border-color 0.15s ease-out',
  },
  secondaryBtnDone: {
    borderColor: 'rgba(20, 184, 166, 0.4)',
    color: '#2dd4bf',
  },
  selectFormat: {
    paddingBlock: '6px',
    paddingInline: '9px',
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
    paddingInline: '13px',
    fontSize: '12px',
    fontWeight: 600,
    color: '#041311',
    backgroundColor: '#14b8a6',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    borderRadius: '6px',
    cursor: 'pointer',
    lineHeight: 1.2,
    transition: 'opacity 0.15s ease-out',
  },
  cancelScanBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '6px',
    paddingInline: '10px',
    fontSize: '12px',
    fontWeight: 600,
    color: '#f87171',
    backgroundColor: 'rgba(239, 68, 68, 0.1)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(239, 68, 68, 0.35)',
    borderRadius: '6px',
    cursor: 'pointer',
    lineHeight: 1.2,
  },
  filterGroup: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '4px',
    backgroundColor: '#171c24',
    padding: '2px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
  },
  filterPill: {
    background: 'none',
    border: 'none',
    color: '#8e98a8',
    fontSize: '11px',
    fontWeight: 500,
    paddingBlock: '3px',
    paddingInline: '8px',
    borderRadius: '4px',
    cursor: 'pointer',
    outline: 'none',
  },
  filterPillActive: {
    backgroundColor: '#232a36',
    color: '#2dd4bf',
    fontWeight: 600,
  },
  disabledControl: {
    opacity: 0.45,
    cursor: 'not-allowed',
  },
});

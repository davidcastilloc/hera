import * as stylex from '@stylexjs/stylex';
import { isTauri } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import type { Playlist, Track } from '../generated/typings';
import type { AgentExecutionPolicy } from '../lib/agent-store';
import type { AgentAttachment } from '../lib/bridge-agent';
import DatabaseBridge from '../lib/bridge-database';
import useLibraryStore from '../lib/store';

interface SlashCommandOption {
  cmd: string;
  label: string;
  template: string;
}

const SLASH_COMMANDS: SlashCommandOption[] = [
  {
    cmd: '/curar-set',
    label: 'Armar playlist armónica y enviarla a VirtualDJ',
    template:
      'Arma una playlist «Warmup Cabina» ordenada por llave Camelot y mándala a VirtualDJ',
  },
  {
    cmd: '/puente',
    label: 'Calcular transición armónica entre dos pistas',
    template:
      'Calcula el mejor puente armónico progresivo entre mis pistas actuales',
  },
  {
    cmd: '/enviar-vdj',
    label: 'Sincronizar selección en carpeta en vivo de VirtualDJ',
    template:
      'Sincroniza mis mejores pistas en la carpeta en vivo de VirtualDJ',
  },
  {
    cmd: '/analizar-carpeta',
    label: 'Escanear e inspeccionar archivos de audio en disco',
    template:
      'Inspecciona los archivos de audio recientes en mi carpeta y resume formatos y estado',
  },
  {
    cmd: '/limpiar-biblioteca',
    label: 'Revisar pistas sin BPM o metadatos incompletos',
    template:
      'Revisa mi biblioteca local y dime qué pistas necesitan normalización o análisis',
  },
  {
    cmd: '/terminal',
    label: 'Ejecutar diagnóstico o comando del sistema (PowerShell)',
    template:
      'Ejecuta un diagnóstico rápido de mi biblioteca local, VirtualDJ y estado del sistema',
  },
];

const PLUS_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.75"
    strokeLinecap="round"
    aria-hidden="true"
  >
    <path d="M12 5v14M5 12h14" />
  </svg>
);

const SEND_UP_SVG = (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M12 19V5M6 11l6-6 6 6" />
  </svg>
);

const STOP_SQUARE_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
  >
    <rect x="5" y="5" width="14" height="14" rx="2.5" />
  </svg>
);

const CLOSE_MINI_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.9"
    strokeLinecap="round"
    aria-hidden="true"
  >
    <path d="M18 6L6 18M6 6l12 12" />
  </svg>
);

const FOLDER_MINI_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4.6l2 2.2h8.4A1.5 1.5 0 0 1 21 9.7v8.8a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5v-11Z" />
  </svg>
);

interface AgentComposerProps {
  value: string;
  attachments: AgentAttachment[];
  backendId: string;
  agyInstalled: boolean;
  executionPolicy: AgentExecutionPolicy;
  isRunning: boolean;
  focusOnMount?: boolean;
  onChange: (value: string) => void;
  onAddAttachment: (att: AgentAttachment) => void;
  onRemoveAttachment: (id: string) => void;
  onBackendChange: (backendId: string) => void;
  onPolicyChange: (policy: AgentExecutionPolicy) => void;
  onSubmit: () => void;
  onStop: () => void;
}

export default function AgentComposer({
  value,
  attachments,
  backendId,
  agyInstalled,
  executionPolicy,
  isRunning,
  focusOnMount = false,
  onChange,
  onAddAttachment,
  onRemoveAttachment,
  onBackendChange,
  onPolicyChange,
  onSubmit,
  onStop,
}: AgentComposerProps) {
  const tracks = useLibraryStore((s) => s.tracks);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [activePopover, setActivePopover] = useState<'none' | 'slash' | 'at'>(
    'none',
  );
  const [popoverQuery, setPopoverQuery] = useState('');

  useEffect(() => {
    if (focusOnMount) {
      textareaRef.current?.focus();
    }
  }, [focusOnMount]);

  useEffect(() => {
    if (activePopover !== 'at') return;
    void DatabaseBridge.getAllPlaylists()
      .then((items) => setPlaylists(items))
      .catch(() => {});
  }, [activePopover]);

  const handleTextChange = useCallback(
    (next: string) => {
      onChange(next);
      startTransition(() => {
        const trimmedStart = next.trimStart();
        if (trimmedStart.startsWith('/') && !trimmedStart.includes(' ')) {
          setActivePopover('slash');
          setPopoverQuery(trimmedStart.slice(1).toLowerCase());
          return;
        }
        const atMatch = /(?:^|\s)@([^\s@]*)$/.exec(next);
        if (atMatch) {
          setActivePopover('at');
          setPopoverQuery((atMatch[1] ?? '').toLowerCase());
          return;
        }
        setActivePopover('none');
      });
    },
    [onChange],
  );

  const filteredSlash = useMemo(() => {
    if (!popoverQuery) return SLASH_COMMANDS;
    return SLASH_COMMANDS.filter(
      (item) =>
        item.cmd.toLowerCase().includes(popoverQuery) ||
        item.label.toLowerCase().includes(popoverQuery),
    );
  }, [popoverQuery]);

  const filteredTracks = useMemo(() => {
    const q = popoverQuery.trim();
    const list = q
      ? tracks.filter(
          (t) =>
            t.title.toLowerCase().includes(q) ||
            (t.artists[0] ?? '').toLowerCase().includes(q) ||
            (t.camelot ?? '').toLowerCase().includes(q),
        )
      : tracks;
    return list.slice(0, 5);
  }, [popoverQuery, tracks]);

  const filteredPlaylists = useMemo(() => {
    const q = popoverQuery.trim();
    const list = q
      ? playlists.filter((p) => p.name.toLowerCase().includes(q))
      : playlists;
    return list.slice(0, 3);
  }, [playlists, popoverQuery]);

  const stripTrailingAtToken = useCallback(
    (raw: string) => raw.replace(/(?:^|\s)@[^\s@]*$/, '').trimEnd(),
    [],
  );

  const handlePickSlash = useCallback(
    (opt: SlashCommandOption) => {
      onChange(opt.template);
      setActivePopover('none');
      textareaRef.current?.focus();
    },
    [onChange],
  );

  const handlePickTrack = useCallback(
    (track: Track) => {
      const artist = track.artists[0] || 'Artista';
      onAddAttachment({
        kind: 'track',
        id: track.id,
        label: `${artist} — ${track.title}`,
        path: track.path,
        meta: `${Math.round(track.bpm ?? 124)} BPM · ${track.camelot ?? '8A'}`,
      });
      if (/(?:^|\s)@[^\s@]*$/.test(value)) {
        onChange(stripTrailingAtToken(value));
      }
      setActivePopover('none');
      textareaRef.current?.focus();
    },
    [onAddAttachment, onChange, stripTrailingAtToken, value],
  );

  const handlePickPlaylist = useCallback(
    (pl: Playlist) => {
      onAddAttachment({
        kind: 'playlist',
        id: pl.id,
        label: pl.name,
        meta: `${pl.tracks.length} pistas`,
      });
      if (/(?:^|\s)@[^\s@]*$/.test(value)) {
        onChange(stripTrailingAtToken(value));
      }
      setActivePopover('none');
      textareaRef.current?.focus();
    },
    [onAddAttachment, onChange, stripTrailingAtToken, value],
  );

  const handlePickFolderFromDisk = useCallback(async () => {
    setActivePopover('none');
    if (!isTauri()) return;
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: 'Adjuntar carpeta para el Agente',
      });
      if (typeof selected === 'string' && selected.trim()) {
        const parts = selected.replace(/\\/g, '/').split('/').filter(Boolean);
        const folderName = parts[parts.length - 1] ?? selected;
        onAddAttachment({
          kind: 'folder',
          id: selected,
          label: folderName,
          path: selected,
        });
      }
    } catch {
      // User cancelled native dialog
    }
  }, [onAddAttachment]);

  const canSend = value.trim().length > 0 || attachments.length > 0;

  return (
    <div {...stylex.props(styles.outerWrap)}>
      {activePopover === 'slash' && filteredSlash.length > 0 ? (
        <div
          role="menu"
          aria-label="Comandos rápidos"
          {...stylex.props(styles.popover)}
        >
          {filteredSlash.map((opt) => (
            <button
              key={opt.cmd}
              type="button"
              onClick={() => handlePickSlash(opt)}
              {...stylex.props(styles.popoverItem)}
            >
              <span {...stylex.props(styles.slashCmdName)}>{opt.cmd}</span>
              <span {...stylex.props(styles.slashCmdDesc)}>{opt.label}</span>
            </button>
          ))}
        </div>
      ) : null}

      {activePopover === 'at' ? (
        <div
          role="menu"
          aria-label="Mencionar contexto"
          {...stylex.props(styles.popover)}
        >
          <button
            type="button"
            onClick={() => void handlePickFolderFromDisk()}
            {...stylex.props(styles.popoverItem)}
          >
            <span {...stylex.props(styles.folderTag)}>{FOLDER_MINI_SVG}</span>
            <span {...stylex.props(styles.slashCmdName)}>
              Elegir carpeta del sistema…
            </span>
          </button>

          {filteredPlaylists.map((pl) => (
            <button
              key={pl.id}
              type="button"
              onClick={() => handlePickPlaylist(pl)}
              {...stylex.props(styles.popoverItem)}
            >
              <span {...stylex.props(styles.mentionKindTag)}>Playlist</span>
              <span {...stylex.props(styles.mentionMainText)}>{pl.name}</span>
              <span {...stylex.props(styles.mentionMetaText)}>
                {pl.tracks.length} pistas
              </span>
            </button>
          ))}

          {filteredTracks.map((track) => (
            <button
              key={track.id}
              type="button"
              onClick={() => handlePickTrack(track)}
              {...stylex.props(styles.popoverItem)}
            >
              <span {...stylex.props(styles.mentionKindTag)}>Pista</span>
              <span {...stylex.props(styles.mentionMainText)}>
                {(track.artists[0] || 'Artista') + ' — ' + track.title}
              </span>
              <span {...stylex.props(styles.mentionMetaText)}>
                {Math.round(track.bpm ?? 124)} · {track.camelot ?? '8A'}
              </span>
            </button>
          ))}
        </div>
      ) : null}

      <div {...stylex.props(styles.composerBox)}>
        {attachments.length > 0 ? (
          <div {...stylex.props(styles.chipsRow)}>
            {attachments.map((att) => (
              <span key={att.id} {...stylex.props(styles.attachmentChip)}>
                <span {...stylex.props(styles.chipLabel)}>@{att.label}</span>
                <button
                  type="button"
                  aria-label={`Quitar ${att.label}`}
                  onClick={() => onRemoveAttachment(att.id)}
                  {...stylex.props(styles.chipRemoveBtn)}
                >
                  {CLOSE_MINI_SVG}
                </button>
              </span>
            ))}
          </div>
        ) : null}

        <textarea
          ref={textareaRef}
          rows={2}
          value={value}
          onChange={(e) => handleTextChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape' && activePopover !== 'none') {
              e.preventDefault();
              setActivePopover('none');
              return;
            }
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              if (!isRunning && canSend) {
                setActivePopover('none');
                onSubmit();
              }
            }
          }}
          placeholder="Pide cualquier acción en tu biblioteca, VirtualDJ o sistema… (/ comandos, @ mencionar)"
          aria-label="Instrucción para el agente"
          {...stylex.props(styles.textarea)}
        />

        <div {...stylex.props(styles.toolbarRow)}>
          <div {...stylex.props(styles.leftTools)}>
            <button
              type="button"
              onClick={() => {
                setPopoverQuery('');
                setActivePopover((prev) => (prev === 'at' ? 'none' : 'at'));
              }}
              title="Mencionar pista, playlist o carpeta (@)"
              {...stylex.props(styles.utilityBtn)}
            >
              {PLUS_SVG}
              <span>Contexto</span>
            </button>

            <button
              type="button"
              onClick={() => {
                setPopoverQuery('');
                setActivePopover((prev) =>
                  prev === 'slash' ? 'none' : 'slash',
                );
              }}
              title="Ver flujos rápidos (/)"
              {...stylex.props(styles.utilityBtn)}
            >
              <span>/ Flujos</span>
            </button>

            <select
              aria-label="Motor del agente"
              value={backendId}
              onChange={(e) => onBackendChange(e.target.value)}
              {...stylex.props(styles.compactSelect)}
            >
              <option value="cli_agy">
                {agyInstalled ? 'agy · Antigravity' : 'agy · Auto'}
              </option>
              <option value="local_fast">Motor Local Instantáneo</option>
              <option value="local_ollama">Ollama Local</option>
            </select>

            <button
              type="button"
              onClick={() =>
                onPolicyChange(executionPolicy === 'auto' ? 'confirm' : 'auto')
              }
              title={
                executionPolicy === 'auto'
                  ? 'Ejecución directa activa (clic para pedir confirmación en terminal/escritura)'
                  : 'Confirmar antes de usar terminal o mover archivos'
              }
              {...stylex.props(
                styles.policyBtn,
                executionPolicy === 'confirm' && styles.policyBtnConfirm,
              )}
            >
              {executionPolicy === 'auto' ? 'Auto' : 'Confirmar'}
            </button>
          </div>

          {isRunning ? (
            <button
              type="button"
              onClick={onStop}
              aria-label="Detener ejecución"
              title="Detener ejecución"
              {...stylex.props(styles.stopCircleBtn)}
            >
              {STOP_SQUARE_SVG}
            </button>
          ) : (
            <button
              type="button"
              disabled={!canSend}
              onClick={() => {
                setActivePopover('none');
                onSubmit();
              }}
              aria-label="Enviar instrucción"
              title="Enviar (Enter)"
              {...stylex.props(
                styles.sendCircleBtn,
                !canSend && styles.sendCircleBtnDisabled,
              )}
            >
              {SEND_UP_SVG}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

const styles = stylex.create({
  outerWrap: {
    position: 'relative',
    width: '100%',
    minWidth: 0,
  },
  popover: {
    position: 'absolute',
    bottom: 'calc(100% + 6px)',
    left: 0,
    right: 0,
    zIndex: 30,
    backgroundColor: '#12161e',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    borderRadius: '10px',
    paddingBlock: '4px',
    paddingInline: '4px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    maxHeight: '232px',
    overflowY: 'auto',
    boxShadow: '0 12px 28px rgba(0, 0, 0, 0.45)',
  },
  popoverItem: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    width: '100%',
    paddingBlock: '7px',
    paddingInline: '10px',
    borderWidth: 0,
    borderRadius: '6px',
    backgroundColor: {
      default: 'transparent',
      ':hover': 'rgba(20, 184, 166, 0.1)',
    },
    color: '#edeff3',
    fontSize: '12.5px',
    textAlign: 'left',
    cursor: 'pointer',
    minWidth: 0,
  },
  slashCmdName: {
    fontWeight: 600,
    color: '#2dd4bf',
    flexShrink: 0,
  },
  slashCmdDesc: {
    color: '#8e98a8',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  folderTag: {
    color: '#2dd4bf',
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
  },
  mentionKindTag: {
    fontSize: '11px',
    fontWeight: 600,
    color: '#8e98a8',
    paddingBlock: '1px',
    paddingInline: '6px',
    borderRadius: '4px',
    backgroundColor: '#19202b',
    flexShrink: 0,
  },
  mentionMainText: {
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
    flexGrow: 1,
  },
  mentionMetaText: {
    color: '#8e98a8',
    fontSize: '11.5px',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  composerBox: {
    display: 'flex',
    flexDirection: 'column',
    backgroundColor: '#141922',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#26303d',
      ':focus-within': 'rgba(20, 184, 166, 0.48)',
    },
    borderRadius: '12px',
    paddingTop: '10px',
    paddingBottom: '8px',
    paddingInline: '12px',
    rowGap: '8px',
    transition: 'border-color 0.15s ease-out',
    minWidth: 0,
  },
  chipsRow: {
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '6px',
    columnGap: '6px',
  },
  attachmentChip: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '2px',
    paddingInline: '8px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    backgroundColor: 'rgba(20, 184, 166, 0.09)',
    color: '#d8f3ee',
    fontSize: '12px',
    maxWidth: '260px',
  },
  chipLabel: {
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  chipRemoveBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 0,
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    cursor: 'pointer',
  },
  textarea: {
    width: '100%',
    minHeight: '44px',
    maxHeight: '140px',
    resize: 'none',
    borderWidth: 0,
    outline: 'none',
    backgroundColor: 'transparent',
    color: '#edeff3',
    fontSize: '13.5px',
    lineHeight: 1.5,
    fontFamily: 'inherit',
    padding: 0,
  },
  toolbarRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
    minHeight: '30px',
    minWidth: 0,
  },
  leftTools: {
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    rowGap: '6px',
    columnGap: '6px',
    minWidth: 0,
  },
  utilityBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    height: '26px',
    paddingInline: '8px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: {
      default: '#10141b',
      ':hover': '#181e29',
    },
    color: '#a3adbd',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  compactSelect: {
    height: '26px',
    paddingInline: '8px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: '#10141b',
    color: '#a3adbd',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
    maxWidth: '168px',
  },
  policyBtn: {
    height: '26px',
    paddingInline: '8px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    backgroundColor: 'rgba(20, 184, 166, 0.08)',
    color: '#2dd4bf',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  policyBtnConfirm: {
    borderColor: 'rgba(245, 158, 11, 0.35)',
    backgroundColor: 'rgba(245, 158, 11, 0.1)',
    color: '#fbbf24',
  },
  sendCircleBtn: {
    width: '30px',
    height: '30px',
    borderRadius: '999px',
    borderWidth: 0,
    backgroundColor: '#14b8a6',
    color: '#071312',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    flexShrink: 0,
    transition: 'opacity 0.15s ease-out',
  },
  sendCircleBtnDisabled: {
    opacity: 0.35,
    cursor: 'default',
  },
  stopCircleBtn: {
    width: '30px',
    height: '30px',
    borderRadius: '999px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(239, 68, 68, 0.45)',
    backgroundColor: 'rgba(239, 68, 68, 0.16)',
    color: '#f87171',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    flexShrink: 0,
  },
});

import * as stylex from '@stylexjs/stylex';
import { open } from '@tauri-apps/plugin-dialog';
import { useEffect, useState } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import type { Config, DiscoveredMusicRoot } from '../generated/typings';
import ConfigBridge from '../lib/bridge-config';
import { configQuery } from '../lib/queries';
import queryClient from '../lib/query-client';
import { discoverWorkableRoots } from '../lib/scanner-v3';
import { removeRedundantFolders } from '../lib/utils-library';
import Icon from './Icon';

export type ConfigWithBoothFields = Config & {
  onboarding_completed?: boolean;
  dj_handle?: string;
};

type Props = {
  isOpen: boolean;
  onClose: () => void;
  onCompleted?: () => void;
};

export async function loadOnboardingDiscoverySafe(): Promise<{
  djHandle: string;
  roots: DiscoveredMusicRoot[];
  existingFolders: string[];
}> {
  let djHandle = 'DaviDJ';
  let existingFolders: string[] = [];
  try {
    const cfg = (await ConfigBridge.getAll()) as ConfigWithBoothFields;
    if (cfg.dj_handle && cfg.dj_handle.trim().length > 0) {
      djHandle = cfg.dj_handle.trim();
    }
    if (Array.isArray(cfg.library_folders)) {
      existingFolders = cfg.library_folders;
    }
  } catch {
    // Default config values
  }
  const roots = await discoverWorkableRoots();
  return { djHandle, roots, existingFolders };
}

async function pickMusicFoldersSafe(): Promise<string[]> {
  try {
    const selected = await open({
      directory: true,
      multiple: true,
      title: 'Seleccionar carpetas de música para cabina',
    });
    if (selected == null) return [];
    return Array.isArray(selected) ? selected : [selected];
  } catch {
    return [];
  }
}

export async function completeOnboardingSetup(params: {
  djHandle: string;
  selectedFolders: string[];
  startScan: boolean;
}): Promise<{
  djHandle: string;
  libraryFolders: string[];
  onboardingCompleted: true;
}> {
  const cleanHandle = params.djHandle.trim() || 'DaviDJ';
  let existingFolders: string[] = [];
  try {
    existingFolders = await ConfigBridge.get('library_folders');
  } catch {
    existingFolders = [];
  }

  const mergedFolders = removeRedundantFolders([
    ...existingFolders,
    ...params.selectedFolders.filter((p) => p.trim().length > 0),
  ]).sort();

  const updates = {
    library_folders: mergedFolders,
    dj_handle: cleanHandle,
    onboarding_completed: true,
  } as Partial<ConfigWithBoothFields>;

  const bridgeAny = ConfigBridge as unknown as {
    multiSet?: (u: Partial<ConfigWithBoothFields>) => Promise<void>;
    set: (k: string, v: unknown) => Promise<void>;
  };

  if (typeof bridgeAny.multiSet === 'function') {
    await bridgeAny.multiSet(updates);
  } else {
    await bridgeAny.set('library_folders', mergedFolders);
    await bridgeAny.set('dj_handle', cleanHandle);
    await bridgeAny.set('onboarding_completed', true);
  }

  await queryClient.invalidateQueries({ queryKey: configQuery.queryKey });

  if (params.startScan && mergedFolders.length > 0) {
    void LibraryAPI.scan().catch(() => {});
  }

  return {
    djHandle: cleanHandle,
    libraryFolders: mergedFolders,
    onboardingCompleted: true,
  };
}

export default function OnboardingWizardModal({
  isOpen,
  onClose,
  onCompleted,
}: Props) {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [djHandle, setDjHandle] = useState('DaviDJ');
  const [discoveredRoots, setDiscoveredRoots] = useState<DiscoveredMusicRoot[]>(
    [],
  );
  const [selectedRoots, setSelectedRoots] = useState<string[]>([]);
  const [manualRoots, setManualRoots] = useState<string[]>([]);
  const [discovering, setDiscovering] = useState(false);
  const [saving, setSaving] = useState(false);
  const [startScan, setStartScan] = useState(true);

  useEffect(() => {
    if (!isOpen) return;
    let cancelled = false;
    setDiscovering(true);
    void loadOnboardingDiscoverySafe().then(
      ({ djHandle: loadedHandle, roots, existingFolders }) => {
        if (cancelled) return;
        setDjHandle(loadedHandle);
        setDiscoveredRoots(roots);
        const preselected = removeRedundantFolders([
          ...existingFolders,
          ...roots.map((r) => r.root_path),
        ]);
        setSelectedRoots(preselected);
        setDiscovering(false);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [isOpen]);

  if (!isOpen) return null;

  const toggleRoot = (rootPath: string) => {
    setSelectedRoots((prev) =>
      prev.includes(rootPath)
        ? prev.filter((p) => p !== rootPath)
        : [...prev, rootPath],
    );
  };

  const handleAddManualFolders = async () => {
    const picked = await pickMusicFoldersSafe();
    if (picked.length === 0) return;
    setManualRoots((prev) => removeRedundantFolders([...prev, ...picked]));
    setSelectedRoots((prev) => removeRedundantFolders([...prev, ...picked]));
  };

  const handleFinish = async () => {
    setSaving(true);
    await completeOnboardingSetup({
      djHandle,
      selectedFolders: selectedRoots,
      startScan,
    });
    setSaving(false);
    onCompleted?.();
    onClose();
  };

  const discoveredPathSet = new Set(discoveredRoots.map((r) => r.root_path));
  const extraManualRoots = manualRoots.filter((p) => !discoveredPathSet.has(p));

  return (
    <div
      {...stylex.props(styles.overlay)}
      role="dialog"
      aria-modal="true"
      aria-labelledby="onboarding-title"
    >
      <div {...stylex.props(styles.card)}>
        <div {...stylex.props(styles.header)}>
          <div {...stylex.props(styles.badgeRow)}>
            <span {...stylex.props(styles.betaBadge)}>HERA v0.7.0 · BETA</span>
            <span {...stylex.props(styles.stepCounter)}>Paso {step} de 3</span>
          </div>
          <button
            type="button"
            onClick={onClose}
            {...stylex.props(styles.closeBtn)}
            aria-label="Cerrar asistente"
          >
            <Icon name="x" size={16} />
          </button>
        </div>

        <div {...stylex.props(styles.body)}>
          {step === 1 && (
            <div {...stylex.props(styles.stepContent)}>
              <div {...stylex.props(styles.iconWrapper)}>
                <Icon name="sparkles" size={28} color="#14b8a6" />
              </div>
              <h2 id="onboarding-title" {...stylex.props(styles.title)}>
                Bienvenido a HERA Studio BETA
              </h2>
              <p {...stylex.props(styles.description)}>
                Motor de curación armónica y DJ Copilot 100% en Rust nativo
                (cero Python). Configura tu alias de cabina antes de indexar tu
                colección.
              </p>

              <label {...stylex.props(styles.fieldLabel)}>
                <span>Nombre / Handle de DJ</span>
                <input
                  type="text"
                  value={djHandle}
                  onChange={(e) => setDjHandle(e.target.value)}
                  placeholder="DaviDJ"
                  aria-label="Nombre de DJ"
                  {...stylex.props(styles.input)}
                />
              </label>

              <div {...stylex.props(styles.alertBox)}>
                <strong {...stylex.props(styles.alertHeading)}>
                  Garantía «Don&apos;t break the user space» (Release BETA):
                </strong>
                <p {...stylex.props(styles.alertText)}>
                  Tu música y videos físicos nunca se alteran, renombran ni
                  borran del disco. Todas las clasificaciones, Mixes (&gt; 10m)
                  y etiquetas Shazam son puramente lógicas.
                </p>
              </div>
            </div>
          )}

          {step === 2 && (
            <div {...stylex.props(styles.stepContent)}>
              <div {...stylex.props(styles.iconWrapper)}>
                <Icon name="disc" size={28} color="#38bdf8" />
              </div>
              <h2 id="onboarding-title" {...stylex.props(styles.title)}>
                Escaneo Inteligente &amp; Criterio de Cabina
              </h2>
              <p {...stylex.props(styles.description)}>
                Detectamos tus bibliotecas de VirtualDJ, Rekordbox y Traktor en
                menos de 1 segundo separando pistas (&lt; 10m) de mixes (&gt;
                10m).
              </p>

              <div {...stylex.props(styles.rootsBox)}>
                {discovering ? (
                  <div {...stylex.props(styles.discoveryMsg)}>
                    Buscando carpetas musicales en bases DJ locales…
                  </div>
                ) : discoveredRoots.length === 0 &&
                  extraManualRoots.length === 0 ? (
                  <div {...stylex.props(styles.discoveryMsg)}>
                    No detectamos bases automáticas aún. Añade una carpeta
                    manualmente o continúa a cabina.
                  </div>
                ) : (
                  <div {...stylex.props(styles.rootsList)}>
                    {discoveredRoots.map((root) => {
                      const checked = selectedRoots.includes(root.root_path);
                      return (
                        <label
                          key={`${root.source_db}-${root.root_path}`}
                          {...stylex.props(styles.rootRow)}
                        >
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={() => toggleRoot(root.root_path)}
                          />
                          <div {...stylex.props(styles.rootMeta)}>
                            <div {...stylex.props(styles.rootTop)}>
                              <span {...stylex.props(styles.rootPath)}>
                                {root.root_path}
                              </span>
                              <span {...stylex.props(styles.sourcePill)}>
                                {root.source_db}
                              </span>
                            </div>
                            <span {...stylex.props(styles.rootStats)}>
                              {root.workable_tracks_count} pistas ·{' '}
                              {root.video_tracks_count} video ·{' '}
                              {root.mixes_over_10m_count} mixes (&gt; 10m) ·{' '}
                              {root.skipped_short_sfx_count} SFX omitidos
                            </span>
                          </div>
                        </label>
                      );
                    })}
                    {extraManualRoots.map((folder) => {
                      const checked = selectedRoots.includes(folder);
                      return (
                        <label key={folder} {...stylex.props(styles.rootRow)}>
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={() => toggleRoot(folder)}
                          />
                          <div {...stylex.props(styles.rootMeta)}>
                            <div {...stylex.props(styles.rootTop)}>
                              <span {...stylex.props(styles.rootPath)}>
                                {folder}
                              </span>
                              <span {...stylex.props(styles.sourcePill)}>
                                Manual
                              </span>
                            </div>
                          </div>
                        </label>
                      );
                    })}
                  </div>
                )}
              </div>

              <button
                type="button"
                onClick={() => void handleAddManualFolders()}
                {...stylex.props(styles.addFolderBtn)}
              >
                <Icon name="folderPlus" size={14} />
                <span>Añadir otra carpeta…</span>
              </button>
            </div>
          )}

          {step === 3 && (
            <div {...stylex.props(styles.stepContent)}>
              <div {...stylex.props(styles.iconWrapper)}>
                <Icon name="bridge" size={28} color="#22c55e" />
              </div>
              <h2 id="onboarding-title" {...stylex.props(styles.title)}>
                Integración de Cabina en Vivo
              </h2>
              <p {...stylex.props(styles.description)}>
                Listo para {djHandle.trim() || 'DaviDJ'} ·{' '}
                {selectedRoots.length} carpeta(s) seleccionada(s) bajo garantía{' '}
                Don&apos;t break the user space.
              </p>
              <div {...stylex.props(styles.cardsGrid)}>
                <div {...stylex.props(styles.miniCard)}>
                  <Icon name="crateList" size={20} color="#14b8a6" />
                  <h4 {...stylex.props(styles.miniCardTitle)}>
                    VirtualDJ Live
                  </h4>
                  <p {...stylex.props(styles.miniCardText)}>
                    Envía crates directamente a tu carpeta VirtualDJ con 8 Hot
                    Cues acústicos listos y Check Pre-Toque.
                  </p>
                </div>
                <div {...stylex.props(styles.miniCard)}>
                  <Icon name="sliders" size={20} color="#f59e0b" />
                  <h4 {...stylex.props(styles.miniCardTitle)}>Modo Mini</h4>
                  <p {...stylex.props(styles.miniCardText)}>
                    Presiona Ctrl+Shift+M para una barra compacta tipo HUD
                    mientras mezclas en pantalla completa.
                  </p>
                </div>
              </div>

              <label {...stylex.props(styles.scanCheckRow)}>
                <input
                  type="checkbox"
                  checked={startScan}
                  onChange={(e) => setStartScan(e.target.checked)}
                />
                <span>
                  Iniciar escaneo térmico al cerrar ({selectedRoots.length}{' '}
                  carpeta(s))
                </span>
              </label>
            </div>
          )}
        </div>

        <div {...stylex.props(styles.footer)}>
          {step > 1 ? (
            <button
              type="button"
              onClick={() => setStep((s) => (s - 1) as 1 | 2)}
              {...stylex.props(styles.secondaryBtn)}
            >
              Anterior
            </button>
          ) : (
            <div />
          )}

          {step < 3 ? (
            <button
              type="button"
              onClick={() => setStep((s) => (s + 1) as 2 | 3)}
              {...stylex.props(styles.primaryBtn)}
            >
              Siguiente
            </button>
          ) : (
            <button
              type="button"
              disabled={saving}
              onClick={() => void handleFinish()}
              {...stylex.props(styles.primaryBtn)}
            >
              {saving ? 'Guardando…' : '¡Comenzar a Mezclar!'}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

const styles = stylex.create({
  overlay: {
    position: 'fixed',
    inset: 0,
    backgroundColor: 'rgba(5, 7, 10, 0.78)',
    backdropFilter: 'blur(6px)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 9999,
    padding: '16px',
  },
  card: {
    width: '100%',
    maxWidth: '560px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '12px',
    boxShadow: '0 20px 48px rgba(0, 0, 0, 0.65)',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '14px',
    paddingInline: '20px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1a202c',
  },
  badgeRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
  },
  betaBadge: {
    fontSize: '11px',
    fontWeight: 700,
    letterSpacing: '0.06em',
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.3)',
    borderRadius: '999px',
    paddingBlock: '2px',
    paddingInline: '8px',
  },
  stepCounter: {
    fontSize: '12px',
    color: '#8b949e',
    fontVariantNumeric: 'tabular-nums',
  },
  closeBtn: {
    backgroundColor: 'transparent',
    borderStyle: 'none',
    color: '#8b949e',
    cursor: 'pointer',
    padding: '4px',
    borderRadius: '6px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  body: {
    paddingBlock: '20px',
    paddingInline: '20px',
    minHeight: '280px',
  },
  stepContent: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    textAlign: 'center',
  },
  iconWrapper: {
    width: '48px',
    height: '48px',
    borderRadius: '50%',
    backgroundColor: 'rgba(255, 255, 255, 0.04)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    marginBottom: '12px',
  },
  title: {
    fontSize: '17px',
    fontWeight: 700,
    color: '#f0f3f6',
    marginTop: 0,
    marginRight: 0,
    marginBottom: '8px',
    marginLeft: 0,
  },
  description: {
    fontSize: '13px',
    color: '#9ba3af',
    lineHeight: 1.5,
    marginTop: 0,
    marginRight: 0,
    marginBottom: '14px',
    marginLeft: 0,
    maxWidth: '470px',
  },
  fieldLabel: {
    width: '100%',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-start',
    rowGap: '6px',
    fontSize: '12px',
    fontWeight: 600,
    color: '#c9d1d9',
    marginBottom: '14px',
  },
  input: {
    width: '100%',
    boxSizing: 'border-box',
    paddingBlock: '8px',
    paddingInline: '10px',
    fontSize: '13px',
    color: '#edeff3',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
  },
  alertBox: {
    backgroundColor: 'rgba(20, 184, 166, 0.06)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.2)',
    borderRadius: '8px',
    paddingBlock: '11px',
    paddingInline: '14px',
    textAlign: 'left',
    width: '100%',
    boxSizing: 'border-box',
  },
  alertHeading: {
    fontSize: '12px',
    color: '#14b8a6',
    display: 'block',
    marginBottom: '4px',
  },
  alertText: {
    fontSize: '12px',
    color: '#8b949e',
    lineHeight: 1.4,
    margin: 0,
  },
  rootsBox: {
    width: '100%',
    backgroundColor: '#10141b',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    paddingBlock: '8px',
    paddingInline: '10px',
    maxHeight: '170px',
    overflowY: 'auto',
    textAlign: 'left',
    boxSizing: 'border-box',
  },
  discoveryMsg: {
    fontSize: '12px',
    color: '#8b949e',
    paddingBlock: '10px',
    textAlign: 'center',
  },
  rootsList: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
  },
  rootRow: {
    display: 'flex',
    alignItems: 'flex-start',
    columnGap: '8px',
    paddingBlock: '6px',
    paddingInline: '8px',
    borderRadius: '6px',
    backgroundColor: '#151a23',
    cursor: 'pointer',
  },
  rootMeta: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    minWidth: 0,
    flexGrow: 1,
  },
  rootTop: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
  },
  rootPath: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  sourcePill: {
    fontSize: '10px',
    fontWeight: 600,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderRadius: '999px',
    paddingBlock: '1px',
    paddingInline: '6px',
    flexShrink: 0,
  },
  rootStats: {
    fontSize: '11px',
    color: '#8b949e',
    fontVariantNumeric: 'tabular-nums',
  },
  addFolderBtn: {
    marginTop: '10px',
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '6px',
    paddingInline: '12px',
    fontSize: '12px',
    fontWeight: 500,
    color: '#edeff3',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  cardsGrid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    rowGap: '12px',
    columnGap: '12px',
    width: '100%',
    marginTop: '4px',
    textAlign: 'left',
  },
  miniCard: {
    backgroundColor: 'rgba(255, 255, 255, 0.02)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '12px',
  },
  miniCardTitle: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#f0f3f6',
    marginTop: '8px',
    marginRight: 0,
    marginBottom: '4px',
    marginLeft: 0,
  },
  miniCardText: {
    fontSize: '11.5px',
    color: '#8b949e',
    margin: 0,
    lineHeight: 1.35,
  },
  scanCheckRow: {
    marginTop: '12px',
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
    fontSize: '12px',
    color: '#c9d1d9',
    cursor: 'pointer',
  },
  footer: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '14px',
    paddingInline: '20px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1a202c',
    backgroundColor: '#090b0e',
  },
  secondaryBtn: {
    backgroundColor: 'transparent',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2d3748',
    color: '#c9d1d9',
    fontSize: '12.5px',
    fontWeight: 500,
    paddingBlock: '6px',
    paddingInline: '14px',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  primaryBtn: {
    backgroundColor: '#14b8a6',
    borderStyle: 'none',
    color: '#041c18',
    fontSize: '12.5px',
    fontWeight: 600,
    paddingBlock: '6px',
    paddingInline: '16px',
    borderRadius: '6px',
    cursor: 'pointer',
  },
});

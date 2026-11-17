import * as stylex from '@stylexjs/stylex';
import { useEffect, useMemo, useState } from 'react';

import {
  type FolderCrateSummary,
  formatFolderCrateSubtitle,
  type OrganizeFolderRequest,
  type OrganizeFolderResponse,
  previewOrApplyOrganize,
  saveCrateRules,
} from '../lib/omkf-bridge';
import Icon from './Icon';

export type CrateRulesBannerProps = {
  crate: FolderCrateSummary;
  rulesMarkdown: string;
  ledgerEntries?: string[];
  onRulesSaved?: (rules: string) => void;
  onOrganized?: () => Promise<unknown> | void;
};

const STRATEGIES: Array<{
  id: OrganizeFolderRequest['strategy'];
  label: string;
  hint: string;
}> = [
  {
    id: 'by_tempo_zone',
    label: 'Por zona de tempo',
    hint: 'Agrupa subcarpetas limpias por zona BPM manteniendo UUID y DSP.',
  },
  {
    id: 'by_genre_era',
    label: 'Por género y época',
    hint: 'Separa familias Genre × Era sin mezclar Salsa con Perreo ni décadas incompatibles.',
  },
  {
    id: 'quarantine_issues',
    label: 'Aislar fallos CD/corruptos',
    hint: 'Mueve únicamente pistas con saltos de CD o corruptas a subcarpeta de revisión.',
  },
];

export function extractLedgerTxIds(
  ledgerEntries?: string[] | string,
): string[] {
  const raw = Array.isArray(ledgerEntries)
    ? ledgerEntries.join('\n')
    : (ledgerEntries ?? '');
  if (!raw) return [];
  const matches = raw.matchAll(/tx:#?([A-Za-z0-9_-]+)/g);
  const seen = new Set<string>();
  const out: string[] = [];
  for (const m of matches) {
    if (m[1] && !seen.has(m[1])) {
      seen.add(m[1]);
      out.push(m[1]);
    }
  }
  return out;
}

async function saveCrateRulesSafe(
  crateId: string,
  rules: string,
): Promise<boolean> {
  try {
    return await saveCrateRules(crateId, rules);
  } catch {
    return false;
  }
}

async function runOrganizeSafe(
  req: OrganizeFolderRequest,
): Promise<{ res: OrganizeFolderResponse | null; error: string | null }> {
  try {
    const res = await previewOrApplyOrganize(req);
    return { res, error: null };
  } catch (err) {
    return {
      res: null,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export default function CrateRulesBanner({
  crate,
  rulesMarkdown,
  ledgerEntries,
  onRulesSaved,
  onOrganized,
}: CrateRulesBannerProps) {
  const initialRules = rulesMarkdown || '';
  const [editorOpen, setEditorOpen] = useState(false);
  const [rulesDraft, setRulesDraft] = useState(initialRules);
  const [savingRules, setSavingRules] = useState(false);
  const [rulesSavedFlash, setRulesSavedFlash] = useState(false);

  const [organizeOpen, setOrganizeOpen] = useState(false);
  const [strategy, setStrategy] =
    useState<OrganizeFolderRequest['strategy']>('by_genre_era');
  const [organizing, setOrganizing] = useState(false);
  const [organizeRes, setOrganizeRes] = useState<OrganizeFolderResponse | null>(
    null,
  );
  const [organizeErr, setOrganizeErr] = useState<string | null>(null);
  const [lastTxId, setLastTxId] = useState<string | null>(null);

  useEffect(() => {
    setRulesDraft(rulesMarkdown || '');
  }, [crate.crate_id, rulesMarkdown]);

  const txIds = useMemo(() => {
    const fromLedger = extractLedgerTxIds(ledgerEntries);
    if (lastTxId && !fromLedger.includes(lastTxId)) {
      return [lastTxId, ...fromLedger];
    }
    return fromLedger;
  }, [ledgerEntries, lastTxId]);

  const handleSaveRules = async () => {
    setSavingRules(true);
    const ok = await saveCrateRulesSafe(crate.crate_id, rulesDraft);
    setSavingRules(false);
    if (ok) {
      setRulesSavedFlash(true);
      onRulesSaved?.(rulesDraft);
      setTimeout(() => setRulesSavedFlash(false), 2200);
    }
  };

  const handleDryRun = async () => {
    setOrganizing(true);
    setOrganizeErr(null);
    const { res, error } = await runOrganizeSafe({
      crate_id: crate.crate_id,
      strategy,
      dry_run: true,
    });
    setOrganizeRes(res);
    setOrganizeErr(error);
    setOrganizing(false);
  };

  const handleApplyOrganize = async () => {
    setOrganizing(true);
    setOrganizeErr(null);
    const { res, error } = await runOrganizeSafe({
      crate_id: crate.crate_id,
      strategy,
      dry_run: false,
    });
    setOrganizeRes(res);
    setOrganizeErr(error);
    if (res?.applied_count && res.applied_count > 0 && res.tx_id) {
      setLastTxId(res.tx_id);
      await onOrganized?.();
    }
    setOrganizing(false);
  };

  const handleRollback = async (txId: string) => {
    setOrganizing(true);
    setOrganizeErr(null);
    const { res, error } = await runOrganizeSafe({
      crate_id: crate.crate_id,
      strategy,
      dry_run: false,
      rollback_tx_id: txId,
    });
    setOrganizeRes(res);
    setOrganizeErr(error);
    if (res?.rolled_back) {
      await onOrganized?.();
    }
    setOrganizing(false);
  };

  const subtitle = formatFolderCrateSubtitle(crate);

  return (
    <section
      {...stylex.props(styles.banner)}
      aria-label="Intención y reglas de cabina"
    >
      <div {...stylex.props(styles.topRow)}>
        <div {...stylex.props(styles.metaCol)}>
          <div {...stylex.props(styles.metaHeadline)}>
            <span {...stylex.props(styles.omkfPill)}>[OMKF .hera]</span>
            <span {...stylex.props(styles.metaText)}>{subtitle}</span>
            {crate.dominant_genre ? (
              <span {...stylex.props(styles.genrePill)}>
                {crate.dominant_genre}
                {crate.era_span ? ` · ${crate.era_span}` : ''}
              </span>
            ) : null}
            {crate.video_count > 0 ? (
              <span {...stylex.props(styles.videoPill)}>
                {crate.video_count} video
              </span>
            ) : null}
          </div>
          <span {...stylex.props(styles.rulesPreview)}>
            {rulesDraft.trim()
              ? rulesDraft.trim().split('\n')[0]
              : '# Intención y Reglas de Cabina: sin reglas personalizadas aún'}
          </span>
        </div>

        <div {...stylex.props(styles.actionsRow)}>
          <button
            type="button"
            onClick={() => setEditorOpen((v) => !v)}
            aria-expanded={editorOpen}
            {...stylex.props(styles.secondaryBtn)}
          >
            <Icon name="sliders" size={14} />
            <span>
              {editorOpen ? 'Ocultar reglas' : 'Reglas (.hera/crate.md)'}
            </span>
          </button>

          <button
            type="button"
            onClick={() => {
              setOrganizeOpen(true);
              void handleDryRun();
            }}
            {...stylex.props(styles.primarySolidBtn)}
          >
            <Icon name="sparkles" size={14} color="#041311" />
            <span>Organizar IA</span>
          </button>
        </div>
      </div>

      {editorOpen && (
        <div {...stylex.props(styles.editorBox)}>
          <label {...stylex.props(styles.editorLabel)}>
            <span># Intención y Reglas de Cabina (.hera/crate.md)</span>
            <textarea
              value={rulesDraft}
              onChange={(e) => setRulesDraft(e.target.value)}
              rows={3}
              placeholder="Ej: Mantener merenguetón 2025 separado de merengue clásico 90s; priorizar video HD en hora pico."
              {...stylex.props(styles.textarea)}
            />
          </label>
          <div {...stylex.props(styles.editorActions)}>
            <span {...stylex.props(styles.sealHint)}>
              Actualiza crate.md sin invalidar tree_seal_64
            </span>
            <button
              type="button"
              disabled={savingRules}
              onClick={() => void handleSaveRules()}
              {...stylex.props(styles.saveBtn)}
            >
              {savingRules
                ? 'Guardando…'
                : rulesSavedFlash
                  ? 'Reglas guardadas'
                  : 'Guardar reglas'}
            </button>
          </div>
        </div>
      )}

      {organizeOpen && (
        <div {...stylex.props(styles.modalBackdrop)}>
          <dialog
            open
            aria-label="Organizar con IA"
            {...stylex.props(styles.modalCard)}
          >
            <div {...stylex.props(styles.modalHeader)}>
              <div>
                <h3 {...stylex.props(styles.modalTitle)}>
                  Organizar carpeta con IA · {crate.display_name}
                </h3>
                <span {...stylex.props(styles.modalSub)}>
                  Preserva UUID y OmkfTrackSlot sin re-escanear DSP · Ledger
                  reversible
                </span>
              </div>
              <button
                type="button"
                onClick={() => setOrganizeOpen(false)}
                aria-label="Cerrar organizador"
                {...stylex.props(styles.closeBtn)}
              >
                <Icon name="x" size={16} />
              </button>
            </div>

            <div {...stylex.props(styles.strategyRow)}>
              {STRATEGIES.map((st) => {
                const active = strategy === st.id;
                return (
                  <button
                    key={st.id}
                    type="button"
                    onClick={() => setStrategy(st.id)}
                    {...stylex.props(
                      styles.strategyBtn,
                      active && styles.strategyBtnActive,
                    )}
                  >
                    {st.label}
                  </button>
                );
              })}
            </div>

            <div {...stylex.props(styles.modalBody)}>
              {organizing ? (
                <div {...stylex.props(styles.modalStatus)}>
                  Calculando plan reversible…
                </div>
              ) : organizeErr ? (
                <div {...stylex.props(styles.errorBox)}>{organizeErr}</div>
              ) : organizeRes ? (
                <>
                  <div {...stylex.props(styles.resultSummary)}>
                    <span>
                      {organizeRes.rolled_back
                        ? `Rollback completado (${organizeRes.applied_count} archivo(s) restaurado(s)).`
                        : organizeRes.dry_run
                          ? `Vista previa: ${organizeRes.moves.length} movimiento(s) sugerido(s).`
                          : `Aplicados ${organizeRes.applied_count} movimiento(s) preservando UUID.`}
                    </span>
                    {organizeRes.tx_id ? (
                      <span {...stylex.props(styles.txBadge)}>
                        tx:#{organizeRes.tx_id}
                      </span>
                    ) : null}
                  </div>
                  {organizeRes.moves.length === 0 ? (
                    <div {...stylex.props(styles.emptyPreview)}>
                      Sin movimientos necesarios para esta estrategia.
                    </div>
                  ) : (
                    <ul {...stylex.props(styles.moveList)}>
                      {organizeRes.moves.slice(0, 40).map((mv) => (
                        <li
                          key={`${mv.track_id}-${mv.target_path}`}
                          {...stylex.props(styles.moveItem)}
                        >
                          <span {...stylex.props(styles.moveReason)}>
                            [{mv.bucket_label}] {mv.reason}
                          </span>
                          <span {...stylex.props(styles.movePath)}>
                            {mv.source_path} → {mv.target_path}
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </>
              ) : null}

              {txIds.length > 0 && (
                <div {...stylex.props(styles.rollbackRow)}>
                  <span {...stylex.props(styles.rollbackLabel)}>
                    # Ledger Reversible:
                  </span>
                  {txIds.slice(0, 3).map((txId) => (
                    <button
                      key={txId}
                      type="button"
                      disabled={organizing}
                      onClick={() => void handleRollback(txId)}
                      {...stylex.props(styles.rollbackBtn)}
                    >
                      Revertir tx:#{txId}
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div {...stylex.props(styles.modalFooter)}>
              <button
                type="button"
                disabled={organizing}
                onClick={() => void handleDryRun()}
                {...stylex.props(styles.secondaryBtn)}
              >
                Simular (Dry-Run)
              </button>
              <button
                type="button"
                disabled={
                  organizing ||
                  !organizeRes ||
                  organizeRes.moves.length === 0 ||
                  (!organizeRes.dry_run && organizeRes.applied_count > 0)
                }
                onClick={() => void handleApplyOrganize()}
                {...stylex.props(styles.saveBtn)}
              >
                Aplicar movimientos
              </button>
            </div>
          </dialog>
        </div>
      )}
    </section>
  );
}

const styles = stylex.create({
  banner: {
    backgroundColor: '#10141b',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    paddingBlock: '8px',
    paddingInline: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
    flexShrink: 0,
  },
  topRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
    flexWrap: 'wrap',
    rowGap: '6px',
  },
  metaCol: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
    minWidth: 0,
    flexGrow: 1,
  },
  metaHeadline: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    flexWrap: 'wrap',
  },
  omkfPill: {
    fontSize: '10px',
    fontWeight: 700,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.3)',
    borderRadius: '999px',
    paddingBlock: '1px',
    paddingInline: '7px',
  },
  metaText: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    fontVariantNumeric: 'tabular-nums',
  },
  genrePill: {
    fontSize: '11px',
    color: '#c9d1d9',
    backgroundColor: '#171c24',
    borderRadius: '4px',
    paddingBlock: '1px',
    paddingInline: '6px',
  },
  videoPill: {
    fontSize: '10.5px',
    color: '#38bdf8',
    backgroundColor: 'rgba(56, 189, 248, 0.1)',
    borderRadius: '4px',
    paddingBlock: '1px',
    paddingInline: '6px',
  },
  rulesPreview: {
    fontSize: '11.5px',
    color: '#8e98a8',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  actionsRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  secondaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '5px',
    paddingInline: '10px',
    fontSize: '11.5px',
    fontWeight: 500,
    color: '#edeff3',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  primarySolidBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '5px',
    paddingInline: '11px',
    fontSize: '11.5px',
    fontWeight: 700,
    color: '#041311',
    backgroundColor: '#14b8a6',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  editorBox: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    paddingTop: '6px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1c222d',
  },
  editorLabel: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '4px',
    fontSize: '11px',
    fontWeight: 600,
    color: '#8e98a8',
  },
  textarea: {
    width: '100%',
    boxSizing: 'border-box',
    paddingBlock: '7px',
    paddingInline: '10px',
    fontSize: '12px',
    color: '#edeff3',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    resize: 'vertical',
  },
  editorActions: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  sealHint: {
    fontSize: '11px',
    color: '#64748b',
  },
  saveBtn: {
    paddingBlock: '5px',
    paddingInline: '12px',
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#041311',
    backgroundColor: '#14b8a6',
    borderStyle: 'none',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  modalBackdrop: {
    position: 'fixed',
    inset: 0,
    backgroundColor: 'rgba(5, 7, 10, 0.72)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 9500,
    padding: '16px',
  },
  modalCard: {
    position: 'relative',
    width: '100%',
    maxWidth: '580px',
    margin: 0,
    padding: 0,
    backgroundColor: '#0c0e12',
    color: '#edeff3',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#283140',
    borderRadius: '10px',
    boxShadow: '0 22px 54px rgba(0, 0, 0, 0.72)',
    overflow: 'hidden',
  },
  modalHeader: {
    display: 'flex',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    paddingBlock: '12px',
    paddingInline: '16px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#202836',
  },
  modalTitle: {
    margin: 0,
    fontSize: '14px',
    fontWeight: 600,
    color: '#edeff3',
  },
  modalSub: {
    fontSize: '11px',
    color: '#8e98a8',
  },
  closeBtn: {
    backgroundColor: 'transparent',
    borderStyle: 'none',
    color: '#8e98a8',
    cursor: 'pointer',
    padding: '4px',
  },
  strategyRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '6px',
    paddingBlock: '8px',
    paddingInline: '16px',
    backgroundColor: '#10141b',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#202836',
  },
  strategyBtn: {
    paddingBlock: '5px',
    paddingInline: '8px',
    fontSize: '11.5px',
    color: '#8e98a8',
    backgroundColor: '#151a23',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  strategyBtnActive: {
    color: '#2dd4bf',
    borderColor: 'rgba(20, 184, 166, 0.45)',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    fontWeight: 600,
  },
  modalBody: {
    paddingBlock: '12px',
    paddingInline: '16px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
  },
  modalStatus: {
    fontSize: '12px',
    color: '#8e98a8',
  },
  errorBox: {
    fontSize: '12px',
    color: '#f87171',
    backgroundColor: 'rgba(239, 68, 68, 0.1)',
    paddingBlock: '8px',
    paddingInline: '10px',
    borderRadius: '6px',
  },
  resultSummary: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
    fontSize: '12px',
    color: '#c9d1d9',
  },
  txBadge: {
    fontSize: '10.5px',
    fontWeight: 600,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderRadius: '4px',
    paddingBlock: '2px',
    paddingInline: '6px',
    fontVariantNumeric: 'tabular-nums',
  },
  emptyPreview: {
    fontSize: '12px',
    color: '#8e98a8',
    backgroundColor: '#10141b',
    paddingBlock: '12px',
    paddingInline: '12px',
    borderRadius: '6px',
    textAlign: 'center',
  },
  moveList: {
    listStyle: 'none',
    margin: 0,
    padding: 0,
    maxHeight: '200px',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
  },
  moveItem: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    paddingBlock: '6px',
    paddingInline: '9px',
    backgroundColor: '#10141b',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
  },
  moveReason: {
    fontSize: '11px',
    fontWeight: 600,
    color: '#2dd4bf',
  },
  movePath: {
    fontSize: '11px',
    color: '#8e98a8',
    wordBreak: 'break-all',
  },
  rollbackRow: {
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    columnGap: '6px',
    rowGap: '6px',
    paddingTop: '6px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1c222d',
  },
  rollbackLabel: {
    fontSize: '11px',
    color: '#8e98a8',
  },
  rollbackBtn: {
    fontSize: '11px',
    color: '#fbbf24',
    backgroundColor: 'rgba(245, 158, 11, 0.1)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(245, 158, 11, 0.3)',
    borderRadius: '4px',
    paddingBlock: '3px',
    paddingInline: '7px',
    cursor: 'pointer',
  },
  modalFooter: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'flex-end',
    columnGap: '8px',
    paddingBlock: '10px',
    paddingInline: '16px',
    backgroundColor: '#10141b',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#202836',
  },
});

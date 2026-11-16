import * as stylex from '@stylexjs/stylex';
import { useState } from 'react';

import type { Track } from '../generated/typings';
import {
  auditPreGigCrate,
  type PreGigAuditReport,
  type PreGigTargetProfile,
} from '../lib/scanner-v3';
import Icon from './Icon';

export type PreGigAuditPopoverProps = {
  tracks: Track[];
  crateName?: string;
};

const PROFILES: Array<{ id: PreGigTargetProfile; label: string }> = [
  { id: 'virtualdj', label: 'VirtualDJ' },
  { id: 'usb_cdj', label: 'USB CDJ' },
  { id: 'denon_cd_mp3', label: 'Denon CD/MP3' },
];

export function formatPreGigIssueKind(kind: string): string {
  switch (kind) {
    case 'cd_skip_glitch':
      return 'Salto CD';
    case 'sample_rate_incompatible':
      return '44.1 kHz';
    case 'video_on_cdj':
      return 'Video en CDJ';
    case 'fake_320kbps':
      return 'Fake 320';
    case 'corrupt_file':
      return 'Corrupto';
    case 'missing_file':
      return 'Faltante';
    default:
      return kind;
  }
}

async function runPreGigAuditSafe(
  profile: PreGigTargetProfile,
  trackIds: string[],
): Promise<{ report: PreGigAuditReport | null; error: string | null }> {
  try {
    const report = await auditPreGigCrate(profile, trackIds);
    return { report, error: null };
  } catch (err) {
    return {
      report: null,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export default function PreGigAuditPopover({
  tracks,
  crateName = 'Crate activo',
}: PreGigAuditPopoverProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [profile, setProfile] = useState<PreGigTargetProfile>('virtualdj');
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<PreGigAuditReport | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const executeAudit = async (nextProfile: PreGigTargetProfile) => {
    setBusy(true);
    setErrorMsg(null);
    const trackIds = tracks.map((t) => t.id);
    const { report: res, error } = await runPreGigAuditSafe(
      nextProfile,
      trackIds,
    );
    setReport(res);
    setErrorMsg(error);
    setBusy(false);
  };

  const handleToggle = () => {
    const nextOpen = !isOpen;
    setIsOpen(nextOpen);
    if (nextOpen) {
      void executeAudit(profile);
    }
  };

  const handleSelectProfile = (nextProfile: PreGigTargetProfile) => {
    setProfile(nextProfile);
    void executeAudit(nextProfile);
  };

  return (
    <div {...stylex.props(styles.wrapper)}>
      <button
        type="button"
        onClick={handleToggle}
        aria-expanded={isOpen}
        aria-haspopup="dialog"
        disabled={tracks.length === 0}
        title="Auditar pistas antes de salir a cabina (VirtualDJ / USB CDJ / Denon CD/MP3)"
        {...stylex.props(
          styles.triggerBtn,
          isOpen && styles.triggerBtnActive,
          tracks.length === 0 && styles.disabledBtn,
        )}
      >
        <Icon
          name="checkCircle"
          size={14}
          color={isOpen ? '#2dd4bf' : undefined}
        />
        <span>Check Pre-Toque</span>
      </button>

      {isOpen && (
        <dialog
          open
          aria-label="Check Pre-Toque"
          {...stylex.props(styles.popover)}
        >
          <div {...stylex.props(styles.header)}>
            <div {...stylex.props(styles.titleGroup)}>
              <span {...stylex.props(styles.title)}>Check Pre-Toque</span>
              <span {...stylex.props(styles.crateBadge)}>{crateName}</span>
            </div>
            <button
              type="button"
              onClick={() => setIsOpen(false)}
              aria-label="Cerrar auditoría pre-toque"
              {...stylex.props(styles.closeBtn)}
            >
              <Icon name="x" size={14} />
            </button>
          </div>

          <div
            {...stylex.props(styles.profileBar)}
            role="tablist"
            aria-label="Perfil de hardware en cabina"
          >
            {PROFILES.map((item) => {
              const active = profile === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  role="tab"
                  aria-selected={active}
                  onClick={() => handleSelectProfile(item.id)}
                  {...stylex.props(
                    styles.profileTab,
                    active && styles.profileTabActive,
                  )}
                >
                  {item.label}
                </button>
              );
            })}
          </div>

          <div {...stylex.props(styles.body)}>
            {busy ? (
              <div {...stylex.props(styles.statusText)}>
                Auditando {tracks.length} pistas para {profile}…
              </div>
            ) : errorMsg ? (
              <div {...stylex.props(styles.errorBox)}>{errorMsg}</div>
            ) : report ? (
              <>
                <div {...stylex.props(styles.metricsRow)}>
                  <div {...stylex.props(styles.metricPill)}>
                    <span {...stylex.props(styles.metricLabel)}>Listas</span>
                    <span {...stylex.props(styles.metricReady)}>
                      {report.ready_count}
                    </span>
                  </div>
                  <div {...stylex.props(styles.metricPill)}>
                    <span {...stylex.props(styles.metricLabel)}>Avisos</span>
                    <span {...stylex.props(styles.metricWarn)}>
                      {report.warning_count}
                    </span>
                  </div>
                  <div {...stylex.props(styles.metricPill)}>
                    <span {...stylex.props(styles.metricLabel)}>Bloqueos</span>
                    <span {...stylex.props(styles.metricBlock)}>
                      {report.blocker_count}
                    </span>
                  </div>
                </div>

                {report.issues.length === 0 ? (
                  <div {...stylex.props(styles.cleanBox)}>
                    Cabina lista: {report.total_tracks} pistas sin incidencias
                    para este equipo.
                  </div>
                ) : (
                  <ul {...stylex.props(styles.issueList)}>
                    {report.issues.slice(0, 30).map((issue) => (
                      <li
                        key={`${issue.track_id}-${issue.issue_code}`}
                        {...stylex.props(styles.issueItem)}
                      >
                        <div {...stylex.props(styles.issueHead)}>
                          <span
                            {...stylex.props(
                              styles.issueBadge,
                              issue.severity === 'error'
                                ? styles.issueBadgeError
                                : styles.issueBadgeWarn,
                            )}
                          >
                            {formatPreGigIssueKind(issue.issue_code)}
                          </span>
                          <span {...stylex.props(styles.issueTrack)}>
                            {issue.title}
                          </span>
                        </div>
                        <span {...stylex.props(styles.issueMessage)}>
                          {issue.detail}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </>
            ) : null}
          </div>

          <div {...stylex.props(styles.footer)}>
            <span {...stylex.props(styles.footerHint)}>
              {report
                ? `${report.total_tracks} pistas verificadas`
                : 'Cero cambios en disco'}
            </span>
            <button
              type="button"
              onClick={() => void executeAudit(profile)}
              disabled={busy}
              {...stylex.props(styles.refreshBtn)}
            >
              {busy ? 'Auditando…' : 'Re-auditar'}
            </button>
          </div>
        </dialog>
      )}
    </div>
  );
}

const styles = stylex.create({
  wrapper: {
    position: 'relative',
    display: 'inline-flex',
    flexShrink: 0,
  },
  triggerBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
    whiteSpace: 'nowrap',
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
  },
  triggerBtnActive: {
    borderColor: 'rgba(20, 184, 166, 0.5)',
    color: '#2dd4bf',
  },
  disabledBtn: {
    opacity: 0.45,
    cursor: 'not-allowed',
  },
  popover: {
    position: 'absolute',
    left: 'auto',
    right: 0,
    top: 'calc(100% + 8px)',
    width: '368px',
    margin: 0,
    padding: 0,
    color: '#edeff3',
    borderRadius: '10px',
    backgroundColor: '#151a23',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#283140',
    boxShadow: '0 18px 42px rgba(0, 0, 0, 0.65)',
    zIndex: 80,
    overflow: 'hidden',
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '10px',
    paddingInline: '14px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#202836',
  },
  titleGroup: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    minWidth: 0,
  },
  title: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#edeff3',
  },
  crateBadge: {
    fontSize: '10.5px',
    color: '#8e98a8',
    backgroundColor: '#10141b',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '999px',
    paddingBlock: '1px',
    paddingInline: '7px',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    maxWidth: '140px',
  },
  closeBtn: {
    backgroundColor: 'transparent',
    borderStyle: 'none',
    color: '#8e98a8',
    cursor: 'pointer',
    padding: '4px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  profileBar: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '4px',
    paddingBlock: '8px',
    paddingInline: '12px',
    backgroundColor: '#10141b',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#202836',
  },
  profileTab: {
    paddingBlock: '5px',
    paddingInline: '6px',
    fontSize: '11.5px',
    fontWeight: 500,
    color: '#8e98a8',
    backgroundColor: 'transparent',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'transparent',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  profileTabActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    borderColor: 'rgba(20, 184, 166, 0.35)',
    color: '#2dd4bf',
    fontWeight: 600,
  },
  body: {
    paddingBlock: '10px',
    paddingInline: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
  },
  statusText: {
    fontSize: '12px',
    color: '#8e98a8',
    paddingBlock: '10px',
  },
  errorBox: {
    fontSize: '12px',
    color: '#f87171',
    backgroundColor: 'rgba(239, 68, 68, 0.1)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(239, 68, 68, 0.3)',
    borderRadius: '6px',
    paddingBlock: '8px',
    paddingInline: '10px',
  },
  metricsRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '6px',
  },
  metricPill: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    rowGap: '2px',
    paddingBlock: '6px',
    paddingInline: '8px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    fontVariantNumeric: 'tabular-nums',
  },
  metricLabel: {
    fontSize: '10.5px',
    color: '#8e98a8',
  },
  metricReady: {
    fontSize: '14px',
    fontWeight: 700,
    color: '#2dd4bf',
  },
  metricWarn: {
    fontSize: '14px',
    fontWeight: 700,
    color: '#fbbf24',
  },
  metricBlock: {
    fontSize: '14px',
    fontWeight: 700,
    color: '#f87171',
  },
  cleanBox: {
    fontSize: '12px',
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.08)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.25)',
    borderRadius: '6px',
    paddingBlock: '10px',
    paddingInline: '12px',
    lineHeight: 1.4,
  },
  issueList: {
    listStyle: 'none',
    margin: 0,
    padding: 0,
    maxHeight: '210px',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
  },
  issueItem: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
    paddingBlock: '7px',
    paddingInline: '9px',
    backgroundColor: '#10141b',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
  },
  issueHead: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    minWidth: 0,
  },
  issueBadge: {
    fontSize: '10px',
    fontWeight: 700,
    paddingBlock: '1px',
    paddingInline: '5px',
    borderRadius: '4px',
    flexShrink: 0,
  },
  issueBadgeError: {
    backgroundColor: 'rgba(239, 68, 68, 0.14)',
    color: '#f87171',
  },
  issueBadgeWarn: {
    backgroundColor: 'rgba(245, 158, 11, 0.14)',
    color: '#fbbf24',
  },
  issueTrack: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  issueMessage: {
    fontSize: '11px',
    color: '#8e98a8',
    lineHeight: 1.35,
  },
  footer: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '8px',
    paddingInline: '14px',
    backgroundColor: '#10141b',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#202836',
  },
  footerHint: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  refreshBtn: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#041311',
    backgroundColor: '#14b8a6',
    borderStyle: 'none',
    borderRadius: '5px',
    paddingBlock: '4px',
    paddingInline: '10px',
    cursor: 'pointer',
  },
});

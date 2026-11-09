import * as stylex from '@stylexjs/stylex';
import { memo, useState } from 'react';

import type { AgentToolStepView } from '../lib/bridge-agent';

interface AgentToolStepRowProps {
  step: AgentToolStepView;
  onApprove?: (stepId: string, approved: boolean) => void;
}

const CHECK_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="#14b8a6"
    strokeWidth="1.9"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  </svg>
);

const SPINNER_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="#14b8a6"
    strokeWidth="1.9"
    strokeLinecap="round"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="8.5" stroke="#26303d" />
    <path d="M12 3.5a8.5 8.5 0 0 1 8.5 8.5" />
  </svg>
);

const ALERT_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="#f59e0b"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="9" />
    <path d="M12 8v4.5M12 16h.01" />
  </svg>
);

const ERROR_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="#ef4444"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M18 6L6 18M6 6l12 12" />
  </svg>
);

const CHEVRON_RIGHT_SVG = (
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M9 6l6 6-6 6" />
  </svg>
);

const CHEVRON_DOWN_SVG = (
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M6 9l6 6 6-6" />
  </svg>
);

function renderStatusIcon(status: AgentToolStepView['status']) {
  if (status === 'running') {
    return <span {...stylex.props(styles.spinWrap)}>{SPINNER_SVG}</span>;
  }
  if (status === 'awaiting_approval') {
    return ALERT_SVG;
  }
  if (status === 'error' || status === 'cancelled') {
    return ERROR_SVG;
  }
  return CHECK_SVG;
}

const AgentToolStepRow = memo(function AgentToolStepRow({
  step,
  onApprove,
}: AgentToolStepRowProps) {
  const [expanded, setExpanded] = useState(false);
  const needsApproval = step.status === 'awaiting_approval';
  const hasDetails = Boolean(step.arguments_preview || step.receipt_text);

  return (
    <div {...stylex.props(styles.wrapper)}>
      <div
        {...stylex.props(
          styles.row,
          needsApproval && styles.rowApproval,
          expanded && styles.rowExpanded,
        )}
      >
        <button
          type="button"
          onClick={() => {
            if (hasDetails) setExpanded((prev) => !prev);
          }}
          aria-expanded={expanded}
          {...stylex.props(styles.rowTrigger)}
        >
          <span {...stylex.props(styles.iconSlot)}>
            {renderStatusIcon(step.status)}
          </span>
          <span {...stylex.props(styles.titleText)}>{step.title}</span>
          {step.summary ? (
            <>
              <span {...stylex.props(styles.dotSep)}>·</span>
              <span {...stylex.props(styles.summaryText)}>{step.summary}</span>
            </>
          ) : null}
          {hasDetails ? (
            <span {...stylex.props(styles.chevronSlot)}>
              {expanded ? CHEVRON_DOWN_SVG : CHEVRON_RIGHT_SVG}
            </span>
          ) : null}
        </button>

        {needsApproval && onApprove ? (
          <div {...stylex.props(styles.approvalActions)}>
            <button
              type="button"
              onClick={() => onApprove(step.id, true)}
              {...stylex.props(styles.approveBtn)}
            >
              Ejecutar
            </button>
            <button
              type="button"
              onClick={() => onApprove(step.id, false)}
              {...stylex.props(styles.rejectBtn)}
            >
              Cancelar
            </button>
          </div>
        ) : null}
      </div>

      {expanded && hasDetails ? (
        <div {...stylex.props(styles.receiptBox)}>
          <div {...stylex.props(styles.receiptHeader)}>
            <span {...stylex.props(styles.toolBadge)}>{step.tool_name}</span>
          </div>
          {step.arguments_preview ? (
            <pre {...stylex.props(styles.monoPre)}>
              {step.arguments_preview}
            </pre>
          ) : null}
          {step.receipt_text ? (
            <pre {...stylex.props(styles.monoOutput)}>{step.receipt_text}</pre>
          ) : null}
        </div>
      ) : null}
    </div>
  );
});

export default AgentToolStepRow;

const spinKeyframes = stylex.keyframes({
  from: { transform: 'rotate(0deg)' },
  to: { transform: 'rotate(360deg)' },
});

const styles = stylex.create({
  wrapper: {
    display: 'flex',
    flexDirection: 'column',
    minWidth: 0,
    width: '100%',
  },
  row: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
    minHeight: '28px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'transparent',
    backgroundColor: {
      default: 'transparent',
      ':hover': 'rgba(237, 239, 243, 0.03)',
    },
    transition: 'background-color 0.12s ease-out, border-color 0.12s ease-out',
    minWidth: 0,
  },
  rowApproval: {
    backgroundColor: 'rgba(245, 158, 11, 0.06)',
    borderColor: 'rgba(245, 158, 11, 0.28)',
    paddingInline: '6px',
  },
  rowExpanded: {
    backgroundColor: 'rgba(237, 239, 243, 0.03)',
  },
  rowTrigger: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '7px',
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    paddingBlock: '4px',
    paddingInline: '6px',
    backgroundColor: 'transparent',
    borderWidth: 0,
    color: '#8e98a8',
    fontSize: '12.5px',
    lineHeight: 1.35,
    textAlign: 'left',
    cursor: 'pointer',
  },
  iconSlot: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '14px',
    height: '14px',
    flexShrink: 0,
  },
  spinWrap: {
    display: 'inline-flex',
    animationName: spinKeyframes,
    animationDuration: '0.9s',
    animationIterationCount: 'infinite',
    animationTimingFunction: 'linear',
  },
  titleText: {
    color: '#d4dae3',
    fontWeight: 500,
    whiteSpace: 'nowrap',
    flexShrink: 0,
  },
  dotSep: {
    color: '#566070',
    flexShrink: 0,
  },
  summaryText: {
    color: '#8e98a8',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
    fontVariantNumeric: 'tabular-nums',
  },
  chevronSlot: {
    marginLeft: 'auto',
    color: '#647084',
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
  },
  approvalActions: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  approveBtn: {
    paddingBlock: '3px',
    paddingInline: '9px',
    borderRadius: '5px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.45)',
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    color: '#2dd4bf',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  rejectBtn: {
    paddingBlock: '3px',
    paddingInline: '8px',
    borderRadius: '5px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    backgroundColor: '#141922',
    color: '#8e98a8',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  receiptBox: {
    marginTop: '4px',
    marginBottom: '6px',
    marginLeft: '20px',
    paddingBlock: '8px',
    paddingInline: '10px',
    borderRadius: '8px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: '#10141b',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    minWidth: 0,
    maxWidth: '100%',
  },
  receiptHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  toolBadge: {
    fontFamily:
      'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '11px',
    color: '#2dd4bf',
  },
  monoPre: {
    margin: 0,
    fontFamily:
      'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '11px',
    lineHeight: 1.45,
    color: '#8e98a8',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-word',
    maxHeight: '96px',
    overflowY: 'auto',
    minWidth: 0,
  },
  monoOutput: {
    margin: 0,
    paddingTop: '6px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1d2430',
    fontFamily:
      'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
    fontSize: '11px',
    lineHeight: 1.45,
    color: '#cfd6e0',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-word',
    maxHeight: '180px',
    overflowY: 'auto',
    minWidth: 0,
  },
});

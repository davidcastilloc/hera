import * as stylex from '@stylexjs/stylex';
import { useEffect, useMemo, useState } from 'react';

import {
  buildFolderTreeRows,
  type FolderCrateSummary,
  formatFolderCrateSubtitle,
  getDefaultExpandedCrateIds,
} from '../lib/omkf-bridge';
import Icon from './Icon';

export type FolderCrateTreeProps = {
  crates: FolderCrateSummary[];
  selectedCrateId: string | null;
  onSelectCrate: (crateId: string) => void;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  totalTrackCount?: number;
};

export default function FolderCrateTree({
  crates,
  selectedCrateId,
  onSelectCrate,
  collapsed,
  onToggleCollapsed,
}: FolderCrateTreeProps) {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(() =>
    getDefaultExpandedCrateIds(crates, selectedCrateId),
  );

  useEffect(() => {
    const required = getDefaultExpandedCrateIds(crates, selectedCrateId);
    if (required.size === 0) return;
    setExpandedIds((prev) => {
      let changed = false;
      const next = new Set(prev);
      for (const id of required) {
        if (!next.has(id)) {
          next.add(id);
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [crates, selectedCrateId]);

  const treeRows = useMemo(
    () => buildFolderTreeRows(crates, expandedIds),
    [crates, expandedIds],
  );

  const toggleExpand = (crateId: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(crateId)) {
        next.delete(crateId);
      } else {
        next.add(crateId);
      }
      return next;
    });
  };

  if (collapsed) {
    return (
      <aside
        aria-label="Árbol de carpetas .hera colapsado"
        {...stylex.props(styles.collapsedRail)}
      >
        <button
          type="button"
          onClick={onToggleCollapsed}
          aria-expanded={false}
          title="Mostrar raíces y carpetas"
          {...stylex.props(styles.railToggleBtn)}
        >
          <Icon name="crateList" size={14} color="#2dd4bf" />
        </button>
      </aside>
    );
  }

  return (
    <aside
      aria-label="Raíces y Carpetas (.hera)"
      {...stylex.props(styles.sidebar)}
    >
      <div {...stylex.props(styles.header)}>
        <span {...stylex.props(styles.headerTitle)}>RAÍCES Y CARPETAS</span>
        <button
          type="button"
          onClick={onToggleCollapsed}
          aria-expanded={true}
          title="Ocultar árbol de carpetas"
          {...stylex.props(styles.collapseBtn)}
        >
          <Icon name="x" size={14} />
        </button>
      </div>

      <div {...stylex.props(styles.list)}>
        {treeRows.map((row) => {
          const { crate, depth, hasChildren, isExpanded } = row;
          const active = selectedCrateId === crate.crate_id;
          const badgeCount =
            crate.track_count > 0
              ? crate.track_count
              : (crate.subfolder_count ?? 0);

          return (
            <div
              key={crate.crate_id}
              style={{ paddingLeft: `${6 + depth * 14}px` }}
              {...stylex.props(
                styles.treeRowShell,
                active && styles.treeRowShellActive,
              )}
            >
              {hasChildren ? (
                <button
                  type="button"
                  aria-label={
                    isExpanded
                      ? `Colapsar ${crate.display_name}`
                      : `Expandir ${crate.display_name}`
                  }
                  aria-expanded={isExpanded}
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleExpand(crate.crate_id);
                  }}
                  {...stylex.props(styles.chevronBtn)}
                >
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    {isExpanded ? (
                      <path d="M6 9l6 6 6-6" />
                    ) : (
                      <path d="M9 6l6 6-6 6" />
                    )}
                  </svg>
                </button>
              ) : (
                <span {...stylex.props(styles.leafMarkerSlot)}>
                  <span
                    {...stylex.props(
                      styles.leafDot,
                      active && styles.leafDotActive,
                    )}
                  />
                </span>
              )}

              <button
                type="button"
                onClick={() => {
                  onSelectCrate(crate.crate_id);
                  if (hasChildren && !isExpanded) {
                    toggleExpand(crate.crate_id);
                  }
                }}
                title={crate.folder_path}
                {...stylex.props(styles.itemBtn)}
              >
                <div {...stylex.props(styles.itemTop)}>
                  <span
                    {...stylex.props(
                      styles.itemName,
                      active && styles.itemNameActive,
                    )}
                  >
                    {crate.display_name}
                  </span>
                  <span
                    {...stylex.props(
                      styles.countPill,
                      active && styles.countPillActive,
                    )}
                  >
                    {badgeCount}
                  </span>
                </div>
                <span {...stylex.props(styles.itemSub)}>
                  {formatFolderCrateSubtitle(crate)}
                </span>
              </button>
            </div>
          );
        })}
      </div>
    </aside>
  );
}

const styles = stylex.create({
  collapsedRail: {
    width: '34px',
    flexShrink: 0,
    backgroundColor: '#10141b',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    paddingTop: '10px',
  },
  railToggleBtn: {
    width: '26px',
    height: '26px',
    borderRadius: '6px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
  },
  sidebar: {
    width: '232px',
    flexShrink: 0,
    backgroundColor: '#10141b',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    minHeight: 0,
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '9px',
    paddingInline: '12px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
  },
  headerTitle: {
    fontSize: '11px',
    fontWeight: 700,
    letterSpacing: '0.06em',
    textTransform: 'uppercase',
    color: '#8e98a8',
  },
  collapseBtn: {
    backgroundColor: 'transparent',
    borderStyle: 'none',
    color: '#8e98a8',
    cursor: 'pointer',
    padding: '2px',
    display: 'flex',
    alignItems: 'center',
  },
  list: {
    flexGrow: 1,
    overflowY: 'auto',
    paddingBlock: '6px',
    paddingInline: '6px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
  },
  treeRowShell: {
    display: 'flex',
    alignItems: 'flex-start',
    columnGap: '4px',
    paddingBlock: '5px',
    paddingRight: '8px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'transparent',
    backgroundColor: {
      default: 'transparent',
      ':hover': 'rgba(237, 239, 243, 0.035)',
    },
    transition: 'background-color 0.14s ease-out, border-color 0.14s ease-out',
  },
  treeRowShellActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    borderColor: 'rgba(20, 184, 166, 0.35)',
  },
  chevronBtn: {
    width: '18px',
    height: '18px',
    marginTop: '1px',
    padding: 0,
    borderWidth: 0,
    borderRadius: '4px',
    backgroundColor: {
      default: 'transparent',
      ':hover': '#171c24',
    },
    color: '#8e98a8',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    flexShrink: 0,
  },
  leafMarkerSlot: {
    width: '18px',
    height: '18px',
    marginTop: '1px',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
  },
  leafDot: {
    width: '5px',
    height: '5px',
    borderRadius: '999px',
    backgroundColor: '#2c3545',
  },
  leafDotActive: {
    width: '6px',
    height: '6px',
    backgroundColor: '#2dd4bf',
  },
  itemBtn: {
    flexGrow: 1,
    minWidth: 0,
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    padding: 0,
    backgroundColor: 'transparent',
    borderStyle: 'none',
    textAlign: 'left',
    cursor: 'pointer',
  },
  itemTop: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '6px',
    width: '100%',
  },
  itemName: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  itemNameActive: {
    color: '#2dd4bf',
  },
  countPill: {
    fontSize: '10.5px',
    fontWeight: 600,
    color: '#8e98a8',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '999px',
    paddingBlock: '0.5px',
    paddingInline: '6px',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  countPillActive: {
    color: '#2dd4bf',
    borderColor: 'rgba(20, 184, 166, 0.35)',
    backgroundColor: 'rgba(12, 14, 18, 0.55)',
  },
  itemSub: {
    fontSize: '10.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
});

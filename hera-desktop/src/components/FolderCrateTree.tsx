import * as stylex from '@stylexjs/stylex';

import {
  type FolderCrateSummary,
  formatFolderCrateSubtitle,
} from '../lib/omkf-bridge';
import Icon from './Icon';

export type FolderCrateTreeProps = {
  crates: FolderCrateSummary[];
  selectedCrateId: string | null;
  onSelectCrate: (crateId: string | null) => void;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  totalTrackCount: number;
};

export default function FolderCrateTree({
  crates,
  selectedCrateId,
  onSelectCrate,
  collapsed,
  onToggleCollapsed,
  totalTrackCount,
}: FolderCrateTreeProps) {
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
          title="Mostrar carpetas .hera"
          {...stylex.props(styles.railToggleBtn)}
        >
          <Icon name="crateList" size={14} color="#2dd4bf" />
        </button>
      </aside>
    );
  }

  return (
    <aside aria-label="Carpetas .hera" {...stylex.props(styles.sidebar)}>
      <div {...stylex.props(styles.header)}>
        <span {...stylex.props(styles.headerTitle)}>Carpetas (.hera)</span>
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
        <button
          type="button"
          onClick={() => onSelectCrate(null)}
          {...stylex.props(
            styles.itemBtn,
            selectedCrateId === null && styles.itemBtnActive,
          )}
        >
          <div {...stylex.props(styles.itemTop)}>
            <span {...stylex.props(styles.itemName)}>Todas las carpetas</span>
            <span {...stylex.props(styles.countPill)}>{totalTrackCount}</span>
          </div>
          <span {...stylex.props(styles.itemSub)}>
            {crates.length} crate(s) detectado(s)
          </span>
        </button>

        {crates.map((crate) => {
          const active = selectedCrateId === crate.crate_id;
          return (
            <button
              key={crate.crate_id}
              type="button"
              onClick={() => onSelectCrate(active ? null : crate.crate_id)}
              title={crate.folder_path}
              {...stylex.props(styles.itemBtn, active && styles.itemBtnActive)}
            >
              <div {...stylex.props(styles.itemTop)}>
                <span {...stylex.props(styles.itemName)}>
                  {crate.display_name}
                </span>
                <span {...stylex.props(styles.countPill)}>
                  {crate.track_count}
                </span>
              </div>
              <span {...stylex.props(styles.itemSub)}>
                {formatFolderCrateSubtitle(crate)}
              </span>
            </button>
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
    width: '220px',
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
    borderBottomColor: '#1c222d',
  },
  headerTitle: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
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
    paddingBlock: '8px',
    paddingInline: '8px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '4px',
  },
  itemBtn: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    paddingBlock: '7px',
    paddingInline: '9px',
    borderRadius: '6px',
    backgroundColor: 'transparent',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'transparent',
    textAlign: 'left',
    cursor: 'pointer',
  },
  itemBtnActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
    borderColor: 'rgba(20, 184, 166, 0.3)',
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
  countPill: {
    fontSize: '10.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
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

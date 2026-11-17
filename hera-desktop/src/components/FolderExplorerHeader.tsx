import * as stylex from '@stylexjs/stylex';
import { Fragment, type ReactNode, useMemo } from 'react';

import {
  buildFolderBreadcrumbs,
  type FolderCrateSummary,
} from '../lib/omkf-bridge';

export type FolderExplorerHeaderProps = {
  crates: FolderCrateSummary[];
  selectedCrateId: string | null;
  subfolders: FolderCrateSummary[];
  trackCount: number;
  searchQuery: string;
  onSelectCrate: (crateId: string) => void;
  onClearSearch: () => void;
  children?: ReactNode;
};

const CHEVRON_RIGHT_SVG = (
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
    <path d="M9 6l6 6-6 6" />
  </svg>
);

const FOLDER_OUTLINE_SVG = (
  <svg
    width="14"
    height="14"
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

export default function FolderExplorerHeader({
  crates,
  selectedCrateId,
  subfolders,
  trackCount,
  searchQuery,
  onSelectCrate,
  onClearSearch,
  children,
}: FolderExplorerHeaderProps) {
  const trimmedSearch = searchQuery.trim();
  const isGlobalSearch = trimmedSearch.length > 0;

  const breadcrumbs = useMemo(
    () => buildFolderBreadcrumbs(crates, selectedCrateId),
    [crates, selectedCrateId],
  );

  if (!isGlobalSearch && breadcrumbs.length === 0 && subfolders.length === 0) {
    return <>{children}</>;
  }

  return (
    <div {...stylex.props(styles.headerStack)}>
      {/* Band 1 — Breadcrumb Bar / Global Search Status */}
      <div
        {...stylex.props(styles.breadcrumbBar)}
        aria-label={
          isGlobalSearch
            ? 'Resultados de búsqueda en todas las carpetas'
            : 'Ruta de carpeta activa'
        }
      >
        {isGlobalSearch ? (
          <>
            <div {...stylex.props(styles.searchHeadlineGroup)}>
              <span {...stylex.props(styles.syncDot)} />
              <span {...stylex.props(styles.searchHeadline)}>
                Resultados en todas las carpetas (sin duplicados)
              </span>
              <span {...stylex.props(styles.searchQueryChip)}>
                «{trimmedSearch}»
              </span>
            </div>

            <div {...stylex.props(styles.rightMetaGroup)}>
              <span {...stylex.props(styles.rightCountText)}>
                {trackCount === 1
                  ? '1 coincidencia'
                  : `${trackCount} coincidencias`}
              </span>
              <button
                type="button"
                onClick={onClearSearch}
                {...stylex.props(styles.clearSearchBtn)}
              >
                Limpiar
              </button>
            </div>
          </>
        ) : (
          <>
            <nav
              aria-label="Ruta de carpetas"
              {...stylex.props(styles.breadcrumbTrail)}
            >
              <span {...stylex.props(styles.routeLabel)}>Ruta:</span>
              {breadcrumbs.map((seg, idx) => {
                const isCurrent = idx === breadcrumbs.length - 1;
                return (
                  <Fragment key={seg.crate_id}>
                    {idx > 0 ? (
                      <span {...stylex.props(styles.chevronSep)}>
                        {CHEVRON_RIGHT_SVG}
                      </span>
                    ) : null}
                    <button
                      type="button"
                      onClick={() => onSelectCrate(seg.crate_id)}
                      aria-current={isCurrent ? 'location' : undefined}
                      title={seg.folder_path}
                      {...stylex.props(
                        styles.crumbBtn,
                        isCurrent && styles.crumbBtnCurrent,
                      )}
                    >
                      {seg.display_name}
                    </button>
                  </Fragment>
                );
              })}
            </nav>

            <div {...stylex.props(styles.rightMetaGroup)}>
              <span {...stylex.props(styles.syncDot)} />
              <span {...stylex.props(styles.rightCountText)}>
                {trackCount === 1 ? '1 pista' : `${trackCount} pistas`} · .hera
                sincronizado
              </span>
            </div>
          </>
        )}
      </div>

      {/* Band 2 — Contextual Crate Rules Banner */}
      {!isGlobalSearch ? children : null}

      {/* Band 3 — Immediate Subfolders Strip */}
      {!isGlobalSearch && subfolders.length > 0 ? (
        <div
          {...stylex.props(styles.subfoldersBar)}
          aria-label="Subcarpetas directas"
        >
          <span {...stylex.props(styles.subfoldersLabel)}>Subcarpetas:</span>
          <div {...stylex.props(styles.subfoldersPills)}>
            {subfolders.map((sub) => {
              const count =
                sub.track_count > 0
                  ? sub.track_count
                  : (sub.subfolder_count ?? 0);
              return (
                <button
                  key={sub.crate_id}
                  type="button"
                  onClick={() => onSelectCrate(sub.crate_id)}
                  title={sub.folder_path}
                  {...stylex.props(styles.subfolderPillBtn)}
                >
                  <span {...stylex.props(styles.subfolderIcon)}>
                    {FOLDER_OUTLINE_SVG}
                  </span>
                  <span {...stylex.props(styles.subfolderName)}>
                    {sub.display_name}
                  </span>
                  <span {...stylex.props(styles.subfolderCount)}>
                    · {count}
                  </span>
                </button>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}

const styles = stylex.create({
  headerStack: {
    display: 'flex',
    flexDirection: 'column',
    flexShrink: 0,
  },
  breadcrumbBar: {
    minHeight: '36px',
    paddingBlock: '6px',
    paddingInline: '14px',
    backgroundColor: '#12161d',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
    rowGap: '6px',
    flexWrap: 'wrap',
  },
  breadcrumbTrail: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '4px',
    rowGap: '4px',
    flexWrap: 'wrap',
    minWidth: 0,
  },
  routeLabel: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#8e98a8',
    marginRight: '2px',
  },
  chevronSep: {
    display: 'inline-flex',
    alignItems: 'center',
    color: '#64748b',
  },
  crumbBtn: {
    paddingBlock: '2px',
    paddingInline: '6px',
    borderRadius: '5px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'transparent',
    backgroundColor: {
      default: 'transparent',
      ':hover': '#171c24',
    },
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
    },
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
    transition: 'background-color 0.14s ease-out, color 0.14s ease-out',
  },
  crumbBtnCurrent: {
    color: '#edeff3',
    fontWeight: 600,
    backgroundColor: '#171c24',
    borderColor: '#232a36',
  },
  searchHeadlineGroup: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    minWidth: 0,
    flexWrap: 'wrap',
  },
  searchHeadline: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
  },
  searchQueryChip: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.3)',
    borderRadius: '4px',
    paddingBlock: '1px',
    paddingInline: '6px',
  },
  rightMetaGroup: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '7px',
    flexShrink: 0,
  },
  syncDot: {
    width: '6px',
    height: '6px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  rightCountText: {
    fontSize: '11.5px',
    fontWeight: 500,
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  clearSearchBtn: {
    paddingBlock: '3px',
    paddingInline: '9px',
    borderRadius: '5px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: {
      default: '#171c24',
      ':hover': '#1d2430',
    },
    color: {
      default: '#edeff3',
      ':hover': '#2dd4bf',
    },
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  subfoldersBar: {
    paddingBlock: '7px',
    paddingInline: '14px',
    backgroundColor: '#10141b',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    rowGap: '6px',
    flexWrap: 'wrap',
  },
  subfoldersLabel: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#8e98a8',
    flexShrink: 0,
  },
  subfoldersPills: {
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    columnGap: '6px',
    rowGap: '6px',
    minWidth: 0,
  },
  subfolderPillBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#232a36',
      ':hover': 'rgba(20, 184, 166, 0.45)',
    },
    backgroundColor: {
      default: '#171c24',
      ':hover': '#1b222d',
    },
    color: '#edeff3',
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
    transition: 'border-color 0.14s ease-out, background-color 0.14s ease-out',
  },
  subfolderIcon: {
    display: 'inline-flex',
    alignItems: 'center',
    color: '#2dd4bf',
    flexShrink: 0,
  },
  subfolderName: {
    color: '#edeff3',
    fontWeight: 600,
    whiteSpace: 'nowrap',
  },
  subfolderCount: {
    color: '#8e98a8',
    fontSize: '11.5px',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  },
});

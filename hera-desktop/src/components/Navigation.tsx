import { NavigationMenu } from '@base-ui/react/navigation-menu';
import * as stylex from '@stylexjs/stylex';
import { Link } from '@tanstack/react-router';
import type { LinkProps } from '@tanstack/react-router';

import { useAgentStore } from '../lib/agent-store';
import useLibraryStore from '../lib/store';
import Icon, { type IconName } from './Icon';
import ProgressBar from './ProgressBar';
import TrackListStatus from './TrackListStatus';

type NavItemProps = {
  to: LinkProps['to'];
  label: string;
  icon: IconName;
  isLiveRunning?: boolean;
};

function NavItem({ to, label, icon, isLiveRunning = false }: NavItemProps) {
  return (
    <NavigationMenu.Item {...stylex.props(styles.navigationItem)}>
      <NavigationMenu.Link
        render={(renderProps) => (
          <Link
            {...renderProps}
            to={to}
            title={label}
            aria-label={label}
            draggable={false}
            {...stylex.props(styles.navigationLink)}
          >
            <Icon name={icon} size={16} xstyle={styles.navIcon} />
            <span {...stylex.props(styles.navLabel)}>{label}</span>
            {isLiveRunning ? (
              <span
                aria-label="Agente trabajando en segundo plano"
                {...stylex.props(styles.liveRunDot)}
              />
            ) : null}
          </Link>
        )}
      />
    </NavigationMenu.Item>
  );
}

export default function Navigation() {
  const isAgentRunning = useAgentStore(
    (state) => state.runningSessionIds.length > 0,
  );

  return (
    <aside {...stylex.props(styles.rail)} aria-label="Navegación principal">
      <div {...stylex.props(styles.topSection)}>
        <div {...stylex.props(styles.brandBar)}>
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            aria-hidden="true"
            {...stylex.props(styles.brandMark)}
          >
            <rect
              x="2.5"
              y="2.5"
              width="19"
              height="19"
              rx="5"
              fill="#12161d"
              stroke="#232a36"
              strokeWidth="1.5"
            />
            <path
              d="M6.5 12h2l1.8-4.5 2.9 9 2.2-6.2 1.4 1.7h1.7"
              fill="none"
              stroke="#14b8a6"
              strokeWidth="1.75"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
          <span {...stylex.props(styles.brandTitle)}>HERA</span>
          <span
            {...stylex.props(styles.brandStatusDot)}
            title="Motor de audio listo"
          />
        </div>

        <NavigationMenu.Root
          orientation="vertical"
          aria-label="Secciones"
          {...stylex.props(styles.navMenuRoot)}
        >
          <NavigationMenu.List {...stylex.props(styles.viewLinks)}>
            <NavItem to="/library" label="Biblioteca" icon="disc" />
            <NavItem
              to="/agent"
              label="Agente"
              icon="sparkles"
              isLiveRunning={isAgentRunning}
            />
            <NavItem to="/artists" label="Artistas" icon="artists" />
            <NavItem to="/playlists" label="Playlists" icon="crateList" />
            <NavItem to="/bridge" label="Transición" icon="bridge" />
            <NavItem to="/dna" label="Mi Estilo" icon="dna" />
            <NavItem to="/settings" label="Ajustes" icon="sliders" />
          </NavigationMenu.List>
        </NavigationMenu.Root>
      </div>

      <div {...stylex.props(styles.footerSection)}>
        <div
          {...stylex.props(styles.statusRow)}
          aria-label="Estado de biblioteca"
        >
          <Status />
        </div>

        <Link
          to="/settings/about"
          draggable={false}
          {...stylex.props(styles.supportLink)}
          title="Acerca de HERA · Proyecto Open Source MIT"
        >
          <Icon name="heart" size={14} />
          <span>Apoyar proyecto</span>
        </Link>
      </div>
    </aside>
  );
}

function Status() {
  const refresh = useLibraryStore((state) => state.refresh);
  const refreshing = useLibraryStore((state) => state.refreshing);
  const status = useLibraryStore((state) => state.tracksStatus);
  const tracks = useLibraryStore((state) => state.tracks);

  const { current, total } = refresh;

  if (refreshing) {
    const isScanning = total === 0;
    const progress = total > 0 ? Math.round((current / total) * 100) : 100;

    return (
      <div {...stylex.props(styles.statusLibraryRefresh)}>
        <div {...stylex.props(styles.statusLibraryRefreshProgress)}>
          {isScanning ? (
            'Escaneando pistas...'
          ) : (
            <ProgressBar
              progress={progress}
              label={total > 0 ? `${current} / ${total}` : ''}
            />
          )}
        </div>
      </div>
    );
  }

  if (status != null) {
    return <TrackListStatus {...status} />;
  }

  if (tracks.length > 0) {
    const totalDuration = tracks.reduce(
      (acc, track) => acc + (track.duration || 0),
      0,
    );
    return <TrackListStatus count={tracks.length} duration={totalDuration} />;
  }

  return <span {...stylex.props(styles.emptyStatus)}>0 pistas · 00:00</span>;
}

const styles = stylex.create({
  rail: {
    width: '196px',
    flexGrow: 0,
    flexShrink: 0,
    flexBasis: '196px',
    backgroundColor: '#10141b',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'space-between',
    paddingBlock: '16px',
    paddingInline: '10px',
    userSelect: 'none',
  },
  topSection: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '18px',
  },
  brandBar: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '2px',
    paddingInline: '10px',
  },
  brandMark: {
    flexShrink: 0,
  },
  brandTitle: {
    fontSize: '13.5px',
    fontWeight: 700,
    letterSpacing: '0.08em',
    color: '#edeff3',
  },
  brandStatusDot: {
    marginLeft: 'auto',
    width: '6px',
    height: '6px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
  },
  navMenuRoot: {
    width: '100%',
  },
  viewLinks: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    listStyle: 'none',
    padding: 0,
    margin: 0,
    width: '100%',
  },
  navigationItem: {
    width: '100%',
  },
  navigationLink: {
    width: '100%',
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    paddingBlock: '8px',
    paddingInline: '10px',
    borderRadius: '6px',
    fontSize: '13px',
    fontWeight: {
      default: 500,
      '[data-status="active"]': 600,
    },
    lineHeight: 1.25,
    textDecoration: 'none',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: 'transparent',
      '[data-status="active"]': 'rgba(20, 184, 166, 0.24)',
    },
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
      ':focus': '#edeff3',
      '[data-status="active"]': '#edeff3',
    },
    backgroundColor: {
      default: 'transparent',
      ':hover': 'rgba(237, 239, 243, 0.04)',
      '[data-status="active"]': 'rgba(20, 184, 166, 0.1)',
    },
    transition:
      'background-color 0.15s ease-out, color 0.15s ease-out, border-color 0.15s ease-out',
  },
  navIcon: {
    color: 'inherit',
  },
  navLabel: {
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  liveRunDot: {
    marginLeft: 'auto',
    width: '6px',
    height: '6px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  footerSection: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    paddingTop: '12px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#232a36',
  },
  statusRow: {
    fontSize: '12px',
    paddingInline: '8px',
    color: '#8e98a8',
    display: 'flex',
    alignItems: 'center',
    minHeight: '18px',
    fontVariantNumeric: 'tabular-nums',
  },
  emptyStatus: {
    fontVariantNumeric: 'tabular-nums',
    color: '#8e98a8',
  },
  supportLink: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '7px',
    paddingBlock: '4px',
    paddingInline: '8px',
    borderRadius: '5px',
    fontSize: '11.5px',
    fontWeight: 500,
    textDecoration: 'none',
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
    },
    transition: 'color 0.15s ease-out',
  },
  statusLibraryRefresh: {
    display: 'flex',
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    alignItems: 'center',
  },
  statusLibraryRefreshProgress: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
  },
});

import { NavigationMenu } from '@base-ui/react/navigation-menu';
import * as stylex from '@stylexjs/stylex';
import { Link, useNavigate } from '@tanstack/react-router';
import type { LinkProps } from '@tanstack/react-router';

import { useHeraViewStore } from '../lib/hera-human-language';
import useLibraryStore from '../lib/store';
import Icon, { type IconName } from './Icon';
import ProgressBar from './ProgressBar';
import TrackListStatus from './TrackListStatus';

type NavItemProps = {
  to: LinkProps['to'];
  label: string;
  icon: IconName;
};

function NavItem({ to, label, icon }: NavItemProps) {
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
          </Link>
        )}
      />
    </NavigationMenu.Item>
  );
}

export default function Navigation() {
  const viewMode = useHeraViewStore((state) => state.viewMode);
  const setViewMode = useHeraViewStore((state) => state.setViewMode);
  const openOnboarding = useHeraViewStore((state) => state.openOnboarding);
  const navigate = useNavigate();

  const handleOpenAiConfig = () => {
    openOnboarding();
    void navigate({ to: '/bridge' });
  };

  return (
    <aside {...stylex.props(styles.rail)} aria-label="Navegación de cabina">
      <div {...stylex.props(styles.topSection)}>
        <div {...stylex.props(styles.brandBar)}>
          <svg
            width="20"
            height="20"
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
            title="Motor DSP activo"
          />
        </div>

        <NavigationMenu.Root
          orientation="vertical"
          aria-label="Navegación principal"
          {...stylex.props(styles.navMenuRoot)}
        >
          <NavigationMenu.List {...stylex.props(styles.viewLinks)}>
            <NavItem to="/library" label="Biblioteca" icon="disc" />
            <NavItem to="/artists" label="Artistas" icon="artists" />
            <NavItem to="/playlists" label="Playlists" icon="crateList" />
            <NavItem to="/bridge" label="Puente y Copiloto" icon="bridge" />
            <NavItem to="/dna" label="Mi ADN" icon="dna" />
            <NavItem to="/settings" label="Ajustes" icon="sliders" />
          </NavigationMenu.List>
        </NavigationMenu.Root>
      </div>

      <div {...stylex.props(styles.footerSection)}>
        <div
          {...stylex.props(styles.modeSegment)}
          role="group"
          aria-label="Modo de lectura"
        >
          <button
            type="button"
            onClick={() => setViewMode('simple')}
            {...stylex.props(
              styles.modeBtn,
              viewMode === 'simple' && styles.modeBtnActive,
            )}
            title="Lenguaje directo de cabina sin tecnicismos"
          >
            Simple
          </button>
          <button
            type="button"
            onClick={() => setViewMode('pro')}
            {...stylex.props(
              styles.modeBtn,
              viewMode === 'pro' && styles.modeBtnActive,
            )}
            title="Métricas técnicas: Camelot, 2-Opt TSP y espectro en kHz"
          >
            Pro
          </button>
        </div>

        <button
          type="button"
          onClick={handleOpenAiConfig}
          {...stylex.props(styles.engineStatusBtn)}
          title="Configurar motor de IA y conexión al enjambre P2P"
        >
          <Icon name="cpu" size={14} color="#2dd4bf" />
          <span {...stylex.props(styles.engineStatusText)}>
            {viewMode === 'simple' ? 'Copiloto y Red' : 'Motor IA · P2P'}
          </span>
        </button>

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
          title="Proyecto Open Source MIT · Apoyar desarrollo"
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

  return <span {...stylex.props(styles.emptyStatus)}>0 pistas · 00:00</span>;
}

const styles = stylex.create({
  rail: {
    width: '216px',
    flexGrow: 0,
    flexShrink: 0,
    flexBasis: '216px',
    backgroundColor: '#10141b',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'space-between',
    paddingBlock: '14px',
    paddingInline: '10px',
    userSelect: 'none',
  },
  topSection: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '14px',
  },
  brandBar: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '4px',
    paddingInline: '8px',
  },
  brandMark: {
    flexShrink: 0,
  },
  brandTitle: {
    fontSize: '13.5px',
    fontWeight: 700,
    letterSpacing: '0.06em',
    color: '#edeff3',
  },
  brandStatusDot: {
    marginLeft: 'auto',
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    boxShadow: '0 0 0 3px rgba(20, 184, 166, 0.16)',
  },
  navMenuRoot: {
    width: '100%',
  },
  viewLinks: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
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
      '[data-status="active"]': 'rgba(20, 184, 166, 0.28)',
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
      '[data-status="active"]': 'rgba(20, 184, 166, 0.12)',
    },
    transition:
      'background-color 0.14s ease, color 0.14s ease, border-color 0.14s ease',
  },
  navIcon: {
    color: 'inherit',
  },
  navLabel: {
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  footerSection: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '9px',
    paddingTop: '12px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#232a36',
  },
  modeSegment: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    columnGap: '3px',
    padding: '3px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
  },
  modeBtn: {
    borderWidth: 0,
    borderRadius: '4px',
    paddingBlock: '5px',
    paddingInline: '8px',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
    backgroundColor: 'transparent',
    color: '#8e98a8',
    transition: 'background-color 0.14s ease, color 0.14s ease',
  },
  modeBtnActive: {
    backgroundColor: '#171c24',
    color: '#2dd4bf',
  },
  engineStatusBtn: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    width: '100%',
    paddingBlock: '7px',
    paddingInline: '10px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    color: '#edeff3',
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
    textAlign: 'left',
  },
  engineStatusText: {
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  statusRow: {
    fontSize: '11.5px',
    paddingInline: '6px',
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
    paddingBlock: '5px',
    paddingInline: '6px',
    borderRadius: '5px',
    fontSize: '11.5px',
    fontWeight: 500,
    textDecoration: 'none',
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
    },
    transition: 'color 0.14s ease',
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

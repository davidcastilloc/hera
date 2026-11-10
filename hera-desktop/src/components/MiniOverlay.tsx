import * as stylex from '@stylexjs/stylex';

import MiniBridgeTab from './mini-overlay/MiniBridgeTab';
import {
  CHEVRON_DOWN_SVG,
  EXPAND_WINDOW_SVG,
  PIN_SVG,
  SPARKLE_MINI_SVG,
} from './mini-overlay/MiniIcons';
import MiniPillHud from './mini-overlay/MiniPillHud';
import MiniSuggestionsTab from './mini-overlay/MiniSuggestionsTab';
import { useMiniOverlayController } from './mini-overlay/useMiniOverlayController';

export default function MiniOverlay() {
  const ctrl = useMiniOverlayController();
  const topSuggestion = ctrl.suggestions[0];

  if (ctrl.layoutMode === 'pill') {
    const isTopSent = topSuggestion
      ? ctrl.sentIds.has(topSuggestion.id)
      : false;
    const isTopPlaying = topSuggestion
      ? ctrl.auditionId === topSuggestion.id
      : false;

    return (
      <MiniPillHud
        currentTrackInfo={ctrl.currentTrackInfo}
        topSuggestion={topSuggestion}
        isTopSent={isTopSent}
        isTopPlaying={isTopPlaying}
        hasBusyAction={Boolean(ctrl.busyId)}
        onToggleAudition={(item) => void ctrl.handleToggleAudition(item)}
        onPushVdj={(item) => void ctrl.handlePushSingleTrackToVdj(item)}
        onExpandToCard={() => void ctrl.expandToCard()}
        onRestoreFullWindow={() => void ctrl.restoreFullWindow()}
      />
    );
  }

  return (
    <section aria-label="Modo Mini HERA" {...stylex.props(styles.cardRoot)}>
      <header {...stylex.props(styles.cardHeader)} data-tauri-drag-region>
        <div {...stylex.props(styles.cardBrandGroup)} data-tauri-drag-region>
          <span {...stylex.props(styles.brandText)} data-tauri-drag-region>
            HERA
          </span>
          <span
            {...stylex.props(styles.emeraldDotSmall)}
            data-tauri-drag-region
          />
        </div>

        <div
          role="tablist"
          aria-label="Modo de sugerencia"
          {...stylex.props(styles.modeSwitch)}
        >
          <button
            type="button"
            role="tab"
            aria-selected={ctrl.activeTab === 'siguientes'}
            onClick={() => ctrl.setActiveTab('siguientes')}
            {...stylex.props(
              styles.modeSwitchBtn,
              ctrl.activeTab === 'siguientes' && styles.modeSwitchBtnActive,
            )}
          >
            Siguientes
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={ctrl.activeTab === 'puente'}
            onClick={() => ctrl.setActiveTab('puente')}
            {...stylex.props(
              styles.modeSwitchBtn,
              ctrl.activeTab === 'puente' && styles.modeSwitchBtnActive,
            )}
          >
            Puente
          </button>
        </div>

        <div {...stylex.props(styles.windowControls)}>
          <button
            type="button"
            aria-label="Abrir en Agente"
            title="Abrir en Agente"
            onClick={() => void ctrl.handleOpenAgentFull()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {SPARKLE_MINI_SVG}
          </button>

          <button
            type="button"
            aria-label="Fijar encima"
            aria-pressed={ctrl.pinned}
            title={
              ctrl.pinned ? 'Siempre visible activo' : 'Fijar siempre visible'
            }
            onClick={() => void ctrl.togglePinned()}
            {...stylex.props(
              styles.headerIconBtn,
              ctrl.pinned && styles.headerIconBtnPinned,
            )}
          >
            {PIN_SVG}
          </button>

          <button
            type="button"
            aria-label="Plegar a barra"
            title="Plegar a barra compacta"
            onClick={() => void ctrl.collapseToPill()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {CHEVRON_DOWN_SVG}
          </button>

          <button
            type="button"
            aria-label="Volver a ventana completa"
            title="Volver a HERA completo"
            onClick={() => void ctrl.restoreFullWindow()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {EXPAND_WINDOW_SVG}
          </button>
        </div>
      </header>

      {ctrl.activeTab === 'siguientes' ? (
        <MiniSuggestionsTab
          tracks={ctrl.tracks}
          currentTrackInfo={ctrl.currentTrackInfo}
          selectedCurrentId={ctrl.selectedCurrentId}
          suggestions={ctrl.suggestions}
          sentIds={ctrl.sentIds}
          busyId={ctrl.busyId}
          auditionId={ctrl.auditionId}
          filterText={ctrl.filterText}
          activePreset={ctrl.activePreset}
          onSelectCurrentId={ctrl.setSelectedCurrentId}
          onPushVdj={(target) => void ctrl.handlePushSingleTrackToVdj(target)}
          onToggleAudition={(target) => void ctrl.handleToggleAudition(target)}
          onFilterChange={ctrl.handleFilterChange}
          onApplyPreset={ctrl.handleApplyPreset}
        />
      ) : (
        <MiniBridgeTab
          tracks={ctrl.tracks}
          bridgeEndpoints={ctrl.bridgeEndpoints}
          bridgeOriginId={ctrl.bridgeOriginId}
          bridgeTargetId={ctrl.bridgeTargetId}
          bridgeSteps={ctrl.bridgeSteps}
          auditionId={ctrl.auditionId}
          busyId={ctrl.busyId}
          bridgeSent={ctrl.bridgeSent}
          onSelectOriginId={ctrl.setBridgeOriginId}
          onSelectTargetId={ctrl.setBridgeTargetId}
          onToggleAudition={(target) => void ctrl.handleToggleAudition(target)}
          onPushBridgeToVdj={() => void ctrl.handlePushBridgeToVdj()}
          onSwapEndpoints={ctrl.toggleBridgeSwapped}
        />
      )}
    </section>
  );
}

const styles = stylex.create({
  cardRoot: {
    boxSizing: 'border-box',
    width: '100%',
    height: '100%',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    borderRadius: '10px',
    display: 'flex',
    flexDirection: 'column',
    color: '#edeff3',
    overflow: 'hidden',
    userSelect: 'none',
  },
  cardHeader: {
    height: '42px',
    flexShrink: 0,
    paddingInline: '12px',
    backgroundColor: '#10141b',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
  },
  cardBrandGroup: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    cursor: 'grab',
    flexShrink: 0,
  },
  brandText: {
    fontSize: '13px',
    fontWeight: 700,
    letterSpacing: '0.03em',
    color: '#edeff3',
  },
  emeraldDotSmall: {
    width: '6px',
    height: '6px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  modeSwitch: {
    display: 'inline-flex',
    alignItems: 'center',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '7px',
    padding: '2px',
    columnGap: '2px',
  },
  modeSwitchBtn: {
    borderWidth: 0,
    borderRadius: '5px',
    paddingBlock: '3px',
    paddingInline: '9px',
    fontSize: '11px',
    fontWeight: 500,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    cursor: 'pointer',
  },
  modeSwitchBtnActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.16)',
    color: '#2dd4bf',
  },
  windowControls: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '3px',
    flexShrink: 0,
  },
  headerIconBtn: {
    width: '24px',
    height: '24px',
    borderRadius: '5px',
    borderWidth: 0,
    backgroundColor: {
      default: 'transparent',
      ':hover': '#171c24',
    },
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
    },
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
  },
  headerIconBtnPinned: {
    color: '#2dd4bf',
  },
});

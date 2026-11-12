import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';

import Icon from '../components/Icon';
import TransitionBridgeStepsCard, {
  TransitionFlowArrow,
} from '../components/transition/TransitionBridgeStepsCard';
import TransitionCommandDock from '../components/transition/TransitionCommandDock';
import TransitionDeckCard from '../components/transition/TransitionDeckCard';
import TransitionSuggestionsSection from '../components/transition/TransitionSuggestionsSection';
import { useTransitionStudio } from '../components/transition/useTransitionStudio';
import UnderConstructionView from '../components/UnderConstructionView';
import View from '../elements/View';
import { useFeatureFlags } from '../lib/feature-flags';

export const Route = createFileRoute('/bridge')({
  component: ViewHarmonicBridgeP2P,
});

function ViewHarmonicBridgeP2P() {
  const isEnabled = useFeatureFlags((s) => s.enableHarmonicBridge);

  if (!isEnabled) {
    return (
      <UnderConstructionView
        title="Transición Armónica"
        description="El motor de transición inteligente está en calibración de DSP para conectar pistas armónicas reales desde tu colección local sin emitir tonos sintéticos ni pistas simuladas."
        flagKey="enableHarmonicBridge"
      />
    );
  }

  return <ActiveHarmonicBridgeStudio />;
}

function ActiveHarmonicBridgeStudio() {
  const studio = useTransitionStudio();

  return (
    <View aria-label="Transición">
      <div {...stylex.props(styles.pageContainer)}>
        <div {...stylex.props(styles.scrollCanvas)}>
          <div {...stylex.props(styles.mainSheet)}>
            <header {...stylex.props(styles.headerRow)}>
              <h1 {...stylex.props(styles.pageTitle)}>Transición</h1>

              <div {...stylex.props(styles.headerActions)}>
                <button
                  type="button"
                  onClick={() => void studio.navigate({ to: '/agent' })}
                  {...stylex.props(styles.secondaryBtn)}
                >
                  <Icon name="sparkles" size={14} />
                  <span>Abrir en Agente</span>
                </button>

                <button
                  type="button"
                  disabled={Boolean(studio.busyAction)}
                  onClick={() => void studio.handleExportBridgeVirtualDj()}
                  {...stylex.props(
                    styles.secondaryBtn,
                    studio.inlineBadge === 'exported' &&
                      styles.secondaryBtnDone,
                  )}
                >
                  <Icon
                    name={
                      studio.inlineBadge === 'exported'
                        ? 'checkCircle'
                        : 'export'
                    }
                    size={14}
                    color={
                      studio.inlineBadge === 'exported' ? '#2dd4bf' : undefined
                    }
                  />
                  <span>
                    {studio.busyAction === 'export_vdj'
                      ? 'Exportando...'
                      : studio.inlineBadge === 'exported'
                        ? 'Exportado'
                        : 'Exportar'}
                  </span>
                </button>

                <button
                  type="button"
                  disabled={Boolean(studio.busyAction)}
                  onClick={() => void studio.handleApproveAndCreatePlaylist()}
                  {...stylex.props(styles.primaryBtn)}
                >
                  <Icon name="checkCircle" size={14} />
                  <span>
                    {studio.busyAction === 'approve_set'
                      ? 'Guardando...'
                      : studio.inlineBadge === 'saved'
                        ? 'Guardada'
                        : 'Guardar playlist'}
                  </span>
                </button>
              </div>
            </header>

            <section
              {...stylex.props(styles.flowGrid)}
              aria-label="Ruta de transición"
            >
              <TransitionDeckCard
                label="Actual"
                tracks={studio.tracks}
                selectedTrackId={studio.originTrackId}
                artist={studio.originArtist}
                title={studio.originTitle}
                bpm={studio.originBpm}
                camelot={studio.originCamelot}
                selectAriaLabel="Seleccionar pista actual"
                bpmAriaLabel="BPM pista actual"
                keyAriaLabel="Tono pista actual"
                fallbackBpm={124}
                onSelectTrack={(id) => studio.handleSelectDeckTrack('A', id)}
                onBpmChange={studio.handleOriginBpmChange}
                onCamelotChange={studio.handleOriginCamelotChange}
                onSwapDecks={studio.handleSwapDecks}
              />

              <TransitionFlowArrow />

              <TransitionBridgeStepsCard
                steps={studio.activeSteps}
                activeAuditionId={studio.activeAuditionId}
                findMatchingLocalTrack={studio.findMatchingLocalTrack}
                onAuditionStep={(key, bpm, camelot, cue, local) =>
                  void studio.handleAuditionStep(key, bpm, camelot, cue, local)
                }
              />

              <TransitionFlowArrow />

              <TransitionDeckCard
                label="Destino"
                tracks={studio.tracks}
                selectedTrackId={studio.targetTrackId}
                artist={studio.targetArtist}
                title={studio.targetTitle}
                bpm={studio.targetBpm}
                camelot={studio.targetCamelot}
                selectAriaLabel="Seleccionar pista destino"
                bpmAriaLabel="BPM pista destino"
                keyAriaLabel="Tono pista destino"
                fallbackBpm={128}
                onSelectTrack={(id) => studio.handleSelectDeckTrack('B', id)}
                onBpmChange={studio.handleTargetBpmChange}
                onCamelotChange={studio.handleTargetCamelotChange}
              />
            </section>

            <TransitionSuggestionsSection
              swarmCatalog={studio.swarmCatalog}
              showMoreSuggestions={studio.showMoreSuggestions}
              busyAction={studio.busyAction}
              addedFingerprints={studio.addedFingerprints}
              onToggleSuggestions={() =>
                studio.setShowMoreSuggestions((prev) => !prev)
              }
              onAcquireCandidate={(item) =>
                void studio.handleAcquireAndInsertCandidate(item)
              }
            />
          </div>
        </div>

        <TransitionCommandDock
          activeSteps={studio.activeSteps}
          originBpm={studio.originBpm}
          targetBpm={studio.targetBpm}
          onApplyCustomSteps={(steps) => studio.setCustomSteps(steps)}
        />
      </div>
    </View>
  );
}

const styles = stylex.create({
  pageContainer: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    width: '100%',
    backgroundColor: '#0c0e12',
    color: '#edeff3',
    overflow: 'hidden',
  },
  scrollCanvas: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    overflowY: 'auto',
    paddingBlock: '30px',
    paddingInline: '28px',
  },
  mainSheet: {
    maxWidth: '980px',
    marginInline: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '30px',
  },
  headerRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '16px',
    paddingBottom: '18px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
  },
  pageTitle: {
    margin: 0,
    fontSize: '22px',
    fontWeight: 600,
    letterSpacing: '-0.02em',
    color: '#edeff3',
  },
  headerActions: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  secondaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '13px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: '#151a22',
    color: '#edeff3',
    fontSize: '12.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  secondaryBtnDone: {
    borderColor: 'rgba(20, 184, 166, 0.4)',
    color: '#2dd4bf',
  },
  primaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '14px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontSize: '12.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  flowGrid: {
    display: 'grid',
    gridTemplateColumns: '210px 24px minmax(0, 1fr) 24px 210px',
    alignItems: 'center',
    columnGap: '10px',
  },
});

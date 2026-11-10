import * as stylex from '@stylexjs/stylex';

import type { SwarmCatalogItem } from '../../lib/bridge-hera';
import Icon from '../Icon';

export type TransitionSuggestionsSectionProps = {
  swarmCatalog: Array<SwarmCatalogItem>;
  showMoreSuggestions: boolean;
  busyAction: string | null;
  addedFingerprints: Array<string>;
  onToggleSuggestions: () => void;
  onAcquireCandidate: (item: SwarmCatalogItem) => void;
};

export default function TransitionSuggestionsSection({
  swarmCatalog,
  showMoreSuggestions,
  busyAction,
  addedFingerprints,
  onToggleSuggestions,
  onAcquireCandidate,
}: TransitionSuggestionsSectionProps) {
  const addedFingerprintSet = new Set(addedFingerprints);

  return (
    <section {...stylex.props(styles.suggestionsSection)}>
      <button
        type="button"
        onClick={onToggleSuggestions}
        {...stylex.props(styles.disclosureToggle)}
      >
        <span>Ver más sugerencias ({swarmCatalog.length})</span>
        <Icon
          name={showMoreSuggestions ? 'chevronUp' : 'chevronDown'}
          size={14}
          color="#8e98a8"
        />
      </button>

      {showMoreSuggestions && (
        <div {...stylex.props(styles.suggestionsList)}>
          {swarmCatalog.slice(0, 4).map((item, idx) => {
            const isBusy = busyAction === item.cas_fingerprint;
            const isAdded = addedFingerprintSet.has(item.cas_fingerprint);
            return (
              <div
                key={item.cas_fingerprint}
                {...stylex.props(
                  styles.suggestionRow,
                  idx > 0 && styles.stepRowBorder,
                )}
              >
                <span {...stylex.props(styles.stepNum)}>
                  {String(idx + 1).padStart(2, '0')}
                </span>
                <span {...stylex.props(styles.stepTitle)}>
                  {item.artist} — {item.title}
                </span>
                <span {...stylex.props(styles.stepMeta)}>
                  {Math.round(item.bpm)} BPM · {item.camelot}
                </span>
                <button
                  type="button"
                  disabled={Boolean(busyAction)}
                  onClick={() => onAcquireCandidate(item)}
                  {...stylex.props(styles.addBtn, isAdded && styles.addBtnDone)}
                >
                  <Icon name={isAdded ? 'checkCircle' : 'plus'} size={12} />
                  <span>
                    {isBusy ? 'Añadiendo...' : isAdded ? 'Añadida' : 'Añadir'}
                  </span>
                </button>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

const styles = stylex.create({
  suggestionsSection: {
    maxWidth: '640px',
    width: '100%',
    marginInline: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
  },
  disclosureToggle: {
    alignSelf: 'flex-start',
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
    borderWidth: 0,
    backgroundColor: 'transparent',
    paddingBlock: '4px',
    paddingInline: '2px',
    color: '#edeff3',
    fontSize: '13px',
    fontWeight: 500,
    fontVariantNumeric: 'tabular-nums',
    cursor: 'pointer',
  },
  suggestionsList: {
    backgroundColor: '#10141b',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    overflow: 'hidden',
  },
  suggestionRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
    paddingBlock: '10px',
    paddingInline: '14px',
    minWidth: 0,
  },
  stepRowBorder: {
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1e2530',
  },
  stepNum: {
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  stepTitle: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    fontSize: '13px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  stepMeta: {
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
    flexShrink: 0,
  },
  addBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    backgroundColor: '#171c24',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
    flexShrink: 0,
  },
  addBtnDone: {
    borderColor: 'rgba(20, 184, 166, 0.4)',
    color: '#2dd4bf',
  },
});

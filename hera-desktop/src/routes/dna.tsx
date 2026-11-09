import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useMemo, useState } from 'react';

import View from '../elements/View';
import DatabaseBridge from '../lib/bridge-database';
import {
  computeDjStyleDna,
  type HeraStyleDnaResponse,
} from '../lib/bridge-hera';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/dna')({
  component: ViewDjStyleDna,
});

const DJ_SOFTWARES = ['VirtualDJ', 'Rekordbox', 'Traktor', 'Serato'] as const;

const CAMELOT_KEYS_MINOR = [
  { code: '1A', pct: '1%' },
  { code: '2A', pct: '2%' },
  { code: '3A', pct: '1%' },
  { code: '4A', pct: '3%' },
  { code: '5A', pct: '4%' },
  { code: '6A', pct: '5%' },
  { code: '7A', pct: '8%' },
  { code: '8A', pct: '34%' },
  { code: '9A', pct: '28%' },
  { code: '10A', pct: '19%' },
  { code: '11A', pct: '7%' },
  { code: '12A', pct: '2%' },
];

type SwitchRowProps = {
  label: string;
  checked: boolean;
  onToggle: () => void;
  isLast?: boolean;
};

function PreferenceSwitchRow({
  label,
  checked,
  onToggle,
  isLast = false,
}: SwitchRowProps) {
  return (
    <div {...stylex.props(styles.prefRow, !isLast && styles.prefRowDivider)}>
      <span {...stylex.props(styles.prefLabel)}>{label}</span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        onClick={onToggle}
        {...stylex.props(styles.switchTrack, checked && styles.switchTrackOn)}
      >
        <span
          {...stylex.props(styles.switchThumb, checked && styles.switchThumbOn)}
        />
      </button>
    </div>
  );
}

function ViewDjStyleDna() {
  const tracks = useLibraryStore((state) => state.tracks);

  const [preferredSoftware, setPreferredSoftware] =
    useState<string>('VirtualDJ');
  const [ruleVocalClash, setRuleVocalClash] = useState(true);
  const [ruleSubBass, setRuleSubBass] = useState(true);
  const [ruleAudioQuality, setRuleAudioQuality] = useState(true);
  const [showKeyGrid, setShowKeyGrid] = useState(false);
  const [dnaProfile, setDnaProfile] = useState<HeraStyleDnaResponse | null>(
    null,
  );

  useEffect(() => {
    if (tracks.length === 0) {
      void DatabaseBridge.getAllTracks()
        .then((dbTracks) => {
          if (dbTracks.length > 0) {
            useLibraryStore.setState({ tracks: dbTracks });
          }
        })
        .catch(() => {});
    }
  }, [tracks.length]);

  const goldenRulesString = useMemo(() => {
    const active: string[] = [];
    if (ruleVocalClash) active.push('Evitar choque de voces');
    if (ruleSubBass) active.push('Mantener graves continuos');
    if (ruleAudioQuality) active.push('Ocultar audio de baja calidad');
    return active.join(' · ');
  }, [ruleAudioQuality, ruleSubBass, ruleVocalClash]);

  useEffect(() => {
    void computeDjStyleDna(
      tracks,
      'DaviDJ',
      preferredSoftware,
      goldenRulesString,
    )
      .then(setDnaProfile)
      .catch(() => {});
  }, [tracks, preferredSoftware, goldenRulesString]);

  const numericAverageBpm = dnaProfile?.overall_average_bpm
    ? Math.round(dnaProfile.overall_average_bpm)
    : 125;
  const averageBpm = `${numericAverageBpm} BPM`;
  const dominantKeys =
    dnaProfile?.dominant_camelot_keys &&
    dnaProfile.dominant_camelot_keys.length > 0
      ? dnaProfile.dominant_camelot_keys.slice(0, 3)
      : ['8A', '9A', '10A'];

  return (
    <View aria-label="Mi Estilo">
      <div {...stylex.props(styles.pageContainer)}>
        <div {...stylex.props(styles.centeredSheet)}>
          <header {...stylex.props(styles.headerRow)}>
            <h1 {...stylex.props(styles.pageTitle)}>Mi Estilo</h1>

            <div
              {...stylex.props(styles.segmentedBar)}
              role="group"
              aria-label="Programa DJ principal"
            >
              {DJ_SOFTWARES.map((sw) => {
                const active = preferredSoftware === sw;
                return (
                  <button
                    key={sw}
                    type="button"
                    onClick={() => setPreferredSoftware(sw)}
                    {...stylex.props(
                      styles.segmentBtn,
                      active && styles.segmentBtnActive,
                    )}
                  >
                    {sw}
                  </button>
                );
              })}
            </div>
          </header>

          <section
            {...stylex.props(styles.statsStrip)}
            aria-label="Resumen de estilo"
          >
            <div {...stylex.props(styles.statCol)}>
              <div {...stylex.props(styles.statValue)}>{averageBpm}</div>
              <div {...stylex.props(styles.statCaption)}>Ritmo habitual</div>
            </div>

            <div {...stylex.props(styles.statCol, styles.statColBorder)}>
              <div {...stylex.props(styles.statValue)}>
                {dominantKeys.join(' · ')}
              </div>
              <div {...stylex.props(styles.statCaption)}>Tonos principales</div>
            </div>

            <div {...stylex.props(styles.statCol, styles.statColBorder)}>
              <div {...stylex.props(styles.statValue)}>Progresiva</div>
              <div {...stylex.props(styles.statCaption)}>Mezcla</div>
            </div>
          </section>

          <section {...stylex.props(styles.sectionBlock)}>
            <div {...stylex.props(styles.sectionHeaderRow)}>
              <h2 {...stylex.props(styles.sectionTitle)}>Balance musical</h2>
              <button
                type="button"
                onClick={() => setShowKeyGrid((prev) => !prev)}
                {...stylex.props(styles.quietLinkBtn)}
              >
                {showKeyGrid ? 'Ocultar tonos' : 'Tonos detallados'}
              </button>
            </div>

            <div {...stylex.props(styles.segmentedMeter)}>
              <div
                {...stylex.props(styles.meterSegment)}
                style={{ width: '62%', backgroundColor: '#14b8a6' }}
              />
              <div
                {...stylex.props(styles.meterSegment)}
                style={{ width: '24%', backgroundColor: '#38bdf8' }}
              />
              <div
                {...stylex.props(styles.meterSegment)}
                style={{ width: '14%', backgroundColor: '#f59e0b' }}
              />
            </div>

            <div {...stylex.props(styles.legendRow)}>
              <span {...stylex.props(styles.legendItem)}>
                <span>Melódico</span>
                <strong {...stylex.props(styles.legendTeal)}>62%</strong>
              </span>
              <span {...stylex.props(styles.legendDot)}>·</span>
              <span {...stylex.props(styles.legendItem)}>
                <span>Hora pico</span>
                <strong {...stylex.props(styles.legendSky)}>24%</strong>
              </span>
              <span {...stylex.props(styles.legendDot)}>·</span>
              <span {...stylex.props(styles.legendItem)}>
                <span>Vocal</span>
                <strong {...stylex.props(styles.legendAmber)}>14%</strong>
              </span>
            </div>

            {showKeyGrid && (
              <div {...stylex.props(styles.camelotGrid)}>
                {CAMELOT_KEYS_MINOR.map((item) => {
                  const active = dominantKeys.includes(item.code);
                  return (
                    <div
                      key={item.code}
                      {...stylex.props(
                        styles.camelotCell,
                        active && styles.camelotCellActive,
                      )}
                    >
                      <span
                        {...stylex.props(
                          styles.camelotCode,
                          active && styles.camelotCodeActive,
                        )}
                      >
                        {item.code}
                      </span>
                      <span {...stylex.props(styles.camelotPct)}>
                        {item.pct}
                      </span>
                    </div>
                  );
                })}
              </div>
            )}
          </section>

          <section {...stylex.props(styles.sectionBlock)}>
            <h2 {...stylex.props(styles.sectionTitle)}>
              Preferencias al ordenar
            </h2>

            <div {...stylex.props(styles.prefsBox)}>
              <PreferenceSwitchRow
                label="Evitar choque de voces"
                checked={ruleVocalClash}
                onToggle={() => setRuleVocalClash((v) => !v)}
              />
              <PreferenceSwitchRow
                label="Mantener graves continuos"
                checked={ruleSubBass}
                onToggle={() => setRuleSubBass((v) => !v)}
              />
              <PreferenceSwitchRow
                label="Ocultar audio de baja calidad"
                checked={ruleAudioQuality}
                onToggle={() => setRuleAudioQuality((v) => !v)}
                isLast
              />
            </div>
          </section>
        </div>
      </div>
    </View>
  );
}

const styles = stylex.create({
  pageContainer: {
    height: '100%',
    width: '100%',
    backgroundColor: '#0c0e12',
    color: '#edeff3',
    overflowY: 'auto',
    paddingBlock: '36px',
    paddingInline: '32px',
  },
  centeredSheet: {
    maxWidth: '720px',
    marginInline: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '34px',
  },
  headerRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    flexWrap: 'wrap',
    rowGap: '12px',
    columnGap: '16px',
    paddingBottom: '20px',
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
  segmentedBar: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '2px',
    padding: '3px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '999px',
  },
  segmentBtn: {
    paddingBlock: '5px',
    paddingInline: '12px',
    borderRadius: '999px',
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
    transition: 'background-color 0.15s ease-out, color 0.15s ease-out',
  },
  segmentBtnActive: {
    backgroundColor: '#1b222d',
    color: '#edeff3',
    fontWeight: 600,
  },
  statsStrip: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    paddingBottom: '28px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
  },
  statCol: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    paddingRight: '20px',
  },
  statColBorder: {
    paddingLeft: '24px',
    borderLeftWidth: '1px',
    borderLeftStyle: 'solid',
    borderLeftColor: '#232a36',
  },
  statValue: {
    fontSize: '26px',
    fontWeight: 600,
    letterSpacing: '-0.02em',
    color: '#edeff3',
    fontVariantNumeric: 'tabular-nums',
    lineHeight: 1.15,
  },
  statCaption: {
    fontSize: '12.5px',
    color: '#8e98a8',
    fontWeight: 400,
  },
  sectionBlock: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '14px',
  },
  sectionHeaderRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  sectionTitle: {
    margin: 0,
    fontSize: '15px',
    fontWeight: 600,
    color: '#edeff3',
  },
  quietLinkBtn: {
    borderWidth: 0,
    backgroundColor: 'transparent',
    paddingBlock: '2px',
    paddingInline: '4px',
    fontSize: '12px',
    fontWeight: 500,
    color: '#8e98a8',
    cursor: 'pointer',
  },
  segmentedMeter: {
    display: 'flex',
    width: '100%',
    height: '12px',
    borderRadius: '999px',
    overflow: 'hidden',
    columnGap: '3px',
    backgroundColor: '#12161d',
  },
  meterSegment: {
    height: '100%',
  },
  legendRow: {
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    columnGap: '10px',
    rowGap: '6px',
    fontSize: '12.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  legendItem: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
  },
  legendDot: {
    color: '#4b5565',
  },
  legendTeal: {
    color: '#2dd4bf',
    fontWeight: 600,
  },
  legendSky: {
    color: '#38bdf8',
    fontWeight: 600,
  },
  legendAmber: {
    color: '#f59e0b',
    fontWeight: 600,
  },
  camelotGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(6, 1fr)',
    rowGap: '8px',
    columnGap: '8px',
    paddingTop: '6px',
  },
  camelotCell: {
    paddingBlock: '8px',
    paddingInline: '6px',
    borderRadius: '6px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    rowGap: '2px',
    fontVariantNumeric: 'tabular-nums',
  },
  camelotCellActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderColor: 'rgba(20, 184, 166, 0.35)',
  },
  camelotCode: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
  },
  camelotCodeActive: {
    color: '#2dd4bf',
  },
  camelotPct: {
    fontSize: '11px',
    color: '#8e98a8',
  },
  prefsBox: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    overflow: 'hidden',
  },
  prefRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '16px',
    paddingBlock: '14px',
    paddingInline: '18px',
  },
  prefRowDivider: {
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1d232e',
  },
  prefLabel: {
    fontSize: '13.5px',
    fontWeight: 500,
    color: '#edeff3',
  },
  switchTrack: {
    position: 'relative',
    width: '38px',
    height: '22px',
    borderRadius: '999px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2c3545',
    backgroundColor: '#1b212c',
    padding: 0,
    cursor: 'pointer',
    flexShrink: 0,
    transition: 'background-color 0.16s ease-out, border-color 0.16s ease-out',
  },
  switchTrackOn: {
    backgroundColor: '#14b8a6',
    borderColor: '#14b8a6',
  },
  switchThumb: {
    position: 'absolute',
    top: '2px',
    left: '2px',
    width: '16px',
    height: '16px',
    borderRadius: '999px',
    backgroundColor: '#edeff3',
    transition: 'transform 0.16s ease-out',
  },
  switchThumbOn: {
    transform: 'translateX(16px)',
    backgroundColor: '#ffffff',
  },
});

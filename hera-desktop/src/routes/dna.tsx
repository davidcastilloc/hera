import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

import View from '../elements/View';
import {
  computeDjStyleDna,
  type HeraStyleDnaResponse,
} from '../lib/bridge-hera';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/dna')({
  component: ViewDjStyleDna,
});

const CAMELOT_WHEEL_KEYS = [
  '8A',
  '9A',
  '10A',
  '11A',
  '12A',
  '1A',
  '2A',
  '3A',
  '4A',
  '5A',
  '6A',
  '7A',
  '8B',
  '9B',
  '10B',
  '11B',
  '12B',
  '1B',
  '2B',
  '3B',
  '4B',
  '5B',
  '6B',
  '7B',
];

const DJ_SOFTWARES = ['VirtualDJ', 'Rekordbox', 'Traktor', 'Serato', 'Ableton'];

function ViewDjStyleDna() {
  const tracks = useLibraryStore((state) => state.tracks);
  const [djHandle, setDjHandle] = useState('DaviDJ');
  const [preferredSoftware, setPreferredSoftware] = useState('VirtualDJ');
  const [ruleOwnProductions, setRuleOwnProductions] = useState(true);
  const [ruleLossless320, setRuleLossless320] = useState(true);
  const [ruleStrictCamelot, setRuleStrictCamelot] = useState(true);
  const [rulePrivateLocalVault, setRulePrivateLocalVault] = useState(true);
  const [dnaProfile, setDnaProfile] = useState<HeraStyleDnaResponse | null>(
    null,
  );

  const composedRules = [
    ruleOwnProductions &&
      'Priorizar mis producciones propias (_mis_producciones)',
    ruleLossless320 && 'Calidad mínima FLAC / 320 kbps verificada por FFT',
    ruleStrictCamelot &&
      'Sinergia armónica Camelot estricta (±1 paso + 4 Hot Cues)',
    rulePrivateLocalVault &&
      'Bóveda Privada Local 100% Gratis (mis dubplates inéditos nunca salen sin permiso)',
  ]
    .filter(Boolean)
    .join(' · ');

  useEffect(() => {
    void computeDjStyleDna(
      tracks,
      djHandle,
      preferredSoftware,
      composedRules,
    ).then(setDnaProfile);
  }, [tracks, djHandle, preferredSoftware, composedRules]);

  const dominantKeys = dnaProfile?.dominant_camelot_keys ?? ['8A', '9A', '10A'];

  return (
    <View>
      <div {...stylex.props(styles.container)}>
        <div {...stylex.props(styles.headerRow)}>
          <div>
            <h2 {...stylex.props(styles.title)}>
              🧬 Mi ADN Musical & Reglas del DJ Brain
            </h2>
            <p {...stylex.props(styles.subtitle)}>
              Auto-perfilado determinista en Rust desde tu disco local +
              sincronización ultraligera en Cloudflare D1 ($0.00)
            </p>
          </div>
          <div {...stylex.props(styles.softwarePills)}>
            {DJ_SOFTWARES.map((sw) => (
              <button
                key={sw}
                type="button"
                onClick={() => setPreferredSoftware(sw)}
                {...stylex.props(
                  styles.softwareBtn,
                  preferredSoftware === sw && styles.softwareBtnActive,
                )}
              >
                🎛️ {sw}
              </button>
            ))}
          </div>
        </div>

        <div {...stylex.props(styles.grid)}>
          <div {...stylex.props(styles.card)}>
            <h3 {...stylex.props(styles.cardTitle)}>
              🎯 Rueda Camelot Dominante & Zonas BPM
            </h3>
            <div {...stylex.props(styles.camelotGrid)}>
              {CAMELOT_WHEEL_KEYS.map((key) => {
                const isDominant = dominantKeys.includes(key);
                return (
                  <div
                    key={key}
                    {...stylex.props(
                      styles.camelotCell,
                      isDominant && styles.camelotCellActive,
                    )}
                  >
                    <strong>{key}</strong>
                    <span {...stylex.props(styles.camelotSub)}>
                      {isDominant ? 'Dominante' : 'Armónica'}
                    </span>
                  </div>
                );
              })}
            </div>

            <div {...stylex.props(styles.statsRow)}>
              <div {...stylex.props(styles.statBox)}>
                <span {...stylex.props(styles.statLabel)}>Zona BPM Líder</span>
                <strong {...stylex.props(styles.statValue)}>
                  {dnaProfile?.dominant_bpm_zone ??
                    'club_afro_guaracha_122_132'}
                </strong>
              </div>
              <div {...stylex.props(styles.statBox)}>
                <span {...stylex.props(styles.statLabel)}>BPM Promedio</span>
                <strong {...stylex.props(styles.statValue)}>
                  {dnaProfile?.overall_average_bpm ?? 124.0} BPM
                </strong>
              </div>
              <div {...stylex.props(styles.statBox)}>
                <span {...stylex.props(styles.statLabel)}>Tonos Menores</span>
                <strong {...stylex.props(styles.statValue)}>
                  {dnaProfile?.minor_key_ratio_pct ?? 84.5}%
                </strong>
              </div>
            </div>
          </div>

          <div {...stylex.props(styles.card)}>
            <h3 {...stylex.props(styles.cardTitle)}>
              🧠 Switches de Reglas de Oro del DJ Brain
            </h3>
            <div {...stylex.props(styles.inputRow)}>
              <label {...stylex.props(styles.label)}>
                Handle Artístico DJ:
                <input
                  type="text"
                  value={djHandle}
                  onChange={(e) => setDjHandle(e.target.value)}
                  {...stylex.props(styles.textInput)}
                />
              </label>
            </div>

            <label {...stylex.props(styles.switchRow)}>
              <input
                type="checkbox"
                checked={ruleOwnProductions}
                onChange={(e) => setRuleOwnProductions(e.target.checked)}
              />
              <span>
                <strong>Priorizar mis producciones propias</strong> (incluir al
                menos 1 tema de <code>_mis_producciones</code> por set)
              </span>
            </label>

            <label {...stylex.props(styles.switchRow)}>
              <input
                type="checkbox"
                checked={ruleLossless320}
                onChange={(e) => setRuleLossless320(e.target.checked)}
              />
              <span>
                <strong>Calidad mínima FLAC / 320 kbps Real</strong> (bloquear
                transcodes falsos con auditoría espectral FFT)
              </span>
            </label>

            <label {...stylex.props(styles.switchRow)}>
              <input
                type="checkbox"
                checked={ruleStrictCamelot}
                onChange={(e) => setRuleStrictCamelot(e.target.checked)}
              />
              <span>
                <strong>Sinergia Camelot + 4 Hot Cues Estrictos</strong>{' '}
                (exportar siempre <code>1:Intro</code>, <code>2:Breakdown</code>
                , <code>3:Drop</code>, <code>4:Outro</code>)
              </span>
            </label>

            <label {...stylex.props(styles.switchRow)}>
              <input
                type="checkbox"
                checked={rulePrivateLocalVault}
                onChange={(e) => setRulePrivateLocalVault(e.target.checked)}
              />
              <span>
                <strong>
                  🔒 Bóveda Privada Local 100% Gratis (Open Source)
                </strong>{' '}
                (proteger mis dubplates y edits locales sin paywall)
              </span>
            </label>

            <div {...stylex.props(styles.promptBox)}>
              <div {...stylex.props(styles.promptHeader)}>
                <span>
                  Prompt Block Persistente (~250 tokens · D1 Sync $0.00)
                </span>
              </div>
              <code {...stylex.props(styles.promptCode)}>
                {dnaProfile?.prompt_block ??
                  `[HERA DJ BRAIN — ${djHandle}] Software: ${preferredSoftware} | Reglas: ${composedRules}`}
              </code>
            </div>
          </div>
        </div>
      </div>
    </View>
  );
}

const styles = stylex.create({
  container: {
    padding: '20px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '16px',
    overflowY: 'auto',
    height: '100%',
  },
  headerRow: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    flexWrap: 'wrap',
    rowGap: '12px',
    columnGap: '12px',
  },
  title: {
    margin: 0,
    fontSize: '18px',
  },
  subtitle: {
    margin: 0,
    marginTop: '4px',
    fontSize: '12px',
    color: 'var(--text-muted)',
  },
  softwarePills: {
    display: 'flex',
    rowGap: '6px',
    columnGap: '6px',
    flexWrap: 'wrap',
  },
  softwareBtn: {
    paddingBlock: '6px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--header-bg)',
    color: 'inherit',
    cursor: 'pointer',
    fontSize: '12px',
  },
  softwareBtnActive: {
    borderColor: 'var(--main-color)',
    backgroundColor: 'rgba(139, 92, 246, 0.18)',
    fontWeight: 'bold',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(360px, 1fr))',
    rowGap: '16px',
    columnGap: '16px',
  },
  card: {
    padding: '16px',
    borderRadius: '8px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--tracks-bg-even)',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  cardTitle: {
    margin: 0,
    fontSize: '14px',
  },
  camelotGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(6, 1fr)',
    rowGap: '6px',
    columnGap: '6px',
  },
  camelotCell: {
    padding: '8px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    fontSize: '12px',
  },
  camelotCellActive: {
    borderColor: 'var(--main-color)',
    backgroundColor: 'rgba(139, 92, 246, 0.22)',
  },
  camelotSub: {
    fontSize: '10px',
    color: 'var(--text-muted)',
  },
  statsRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    rowGap: '8px',
    columnGap: '8px',
  },
  statBox: {
    padding: '8px',
    borderRadius: '6px',
    backgroundColor: 'var(--header-bg)',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
  },
  statLabel: {
    fontSize: '10px',
    color: 'var(--text-muted)',
  },
  statValue: {
    fontSize: '12px',
  },
  inputRow: {
    display: 'flex',
    alignItems: 'center',
  },
  label: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    fontSize: '12px',
    width: '100%',
  },
  textInput: {
    paddingBlock: '4px',
    paddingInline: '8px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
    backgroundColor: 'var(--header-bg)',
    color: 'inherit',
    flexGrow: 1,
  },
  switchRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    fontSize: '12px',
    cursor: 'pointer',
  },
  promptBox: {
    padding: '10px',
    borderRadius: '6px',
    backgroundColor: 'var(--header-bg)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'var(--border-color)',
  },
  promptHeader: {
    fontSize: '10px',
    color: 'var(--text-muted)',
    marginBottom: '4px',
  },
  promptCode: {
    fontSize: '11px',
    display: 'block',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-word',
  },
});

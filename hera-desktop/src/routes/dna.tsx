import * as stylex from '@stylexjs/stylex';
import { createFileRoute } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

import Icon from '../components/Icon';
import View from '../elements/View';
import {
  computeDjStyleDna,
  type HeraStyleDnaResponse,
} from '../lib/bridge-hera';
import { useHeraViewStore } from '../lib/hera-human-language';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/dna')({
  component: ViewDjStyleDna,
});

const DJ_SOFTWARES = ['VirtualDJ', 'Rekordbox', 'Traktor', 'Serato', 'Ableton'];

const PRO_CAMELOT_KEYS_MINOR = [
  { code: '1A', note: 'Abm', pct: '1%' },
  { code: '2A', note: 'Ebm', pct: '2%' },
  { code: '3A', note: 'Bbm', pct: '1%' },
  { code: '4A', note: 'Fm', pct: '3%' },
  { code: '5A', note: 'Cm', pct: '4%' },
  { code: '6A', note: 'Gm', pct: '5%' },
  { code: '7A', note: 'Dm', pct: '8%' },
  { code: '8A', note: 'Am', pct: '34%' },
  { code: '9A', note: 'Em', pct: '28%' },
  { code: '10A', note: 'Bm', pct: '19%' },
  { code: '11A', note: 'F#m', pct: '7%' },
  { code: '12A', note: 'Dbm', pct: '2%' },
];

function ViewDjStyleDna() {
  const tracks = useLibraryStore((state) => state.tracks);
  const viewMode = useHeraViewStore((state) => state.viewMode);

  const [djHandle] = useState('DaviDJ');
  const [preferredSoftware, setPreferredSoftware] = useState('VirtualDJ');
  const [ruleVocalClash, setRuleVocalClash] = useState(true);
  const [ruleSubBass, setRuleSubBass] = useState(true);
  const [ruleAudioQuality, setRuleAudioQuality] = useState(true);
  const [dnaProfile, setDnaProfile] = useState<HeraStyleDnaResponse | null>(
    null,
  );

  useEffect(() => {
    void computeDjStyleDna(
      tracks,
      djHandle,
      preferredSoftware,
      'Cuidar vocales · Proteger sub-bajo · Calidad estudio',
    ).then(setDnaProfile);
  }, [tracks, djHandle, preferredSoftware]);

  const analyzedCount = tracks.length;
  const averageBpm = dnaProfile?.overall_average_bpm
    ? `${Math.round(dnaProfile.overall_average_bpm)} BPM`
    : '125 BPM';

  return (
    <View aria-label="Perfil acústico y ADN DJ">
      <div {...stylex.props(styles.pageContainer)}>
        <div {...stylex.props(styles.headerBar)}>
          <div {...stylex.props(styles.titleBlock)}>
            <div {...stylex.props(styles.titleRow)}>
              <Icon name="dna" size={18} color="#2dd4bf" />
              <h1 {...stylex.props(styles.pageTitle)}>
                {viewMode === 'simple'
                  ? 'Mi ADN Musical y Reglas de Mezcla'
                  : 'Perfil Acústico DJ y Matriz Armónica Camelot'}
              </h1>
            </div>
            <p {...stylex.props(styles.subDesc)}>
              {analyzedCount > 0
                ? `Calculado sobre ${analyzedCount} pistas locales en tu biblioteca`
                : 'Perfil base de cabina listo · Importa tus pistas para calibrar tu huella musical'}
            </p>
          </div>

          <div {...stylex.props(styles.switchesGroup)}>
            <span {...stylex.props(styles.deckLabel)}>Software principal:</span>
            {DJ_SOFTWARES.map((sw) => (
              <button
                key={sw}
                type="button"
                onClick={() => setPreferredSoftware(sw)}
                {...stylex.props(
                  styles.swBtn,
                  preferredSoftware === sw && styles.swBtnActive,
                )}
              >
                {sw}
              </button>
            ))}
          </div>
        </div>

        <div {...stylex.props(styles.dnaWorkspace)}>
          <section {...stylex.props(styles.dnaLeft)}>
            <div {...stylex.props(styles.telemetryRow)}>
              <div {...stylex.props(styles.telemetryCard)}>
                <div {...stylex.props(styles.tVal, styles.tValEmerald)}>
                  {averageBpm}
                </div>
                <div {...stylex.props(styles.tName)}>Ritmo dominante</div>
                <div {...stylex.props(styles.tSub)}>
                  Zona habitual entre 122 y 128 BPM
                </div>
              </div>

              <div {...stylex.props(styles.telemetryCard)}>
                <div {...stylex.props(styles.tVal)}>Subida progresiva</div>
                <div {...stylex.props(styles.tName)}>Curva de mezcla</div>
                <div {...stylex.props(styles.tSub)}>
                  Transiciones graduales sin saltos bruscos
                </div>
              </div>

              <div {...stylex.props(styles.telemetryCard)}>
                <div {...stylex.props(styles.tVal)}>Melódica de club</div>
                <div {...stylex.props(styles.tName)}>Identidad sonora</div>
                <div {...stylex.props(styles.tSub)}>
                  {dnaProfile?.dominant_camelot_keys?.slice(0, 3).join(' · ') ||
                    '8A · 9A · 10A'}
                </div>
              </div>
            </div>

            {viewMode === 'simple' ? (
              <div {...stylex.props(styles.panelCard)}>
                <div {...stylex.props(styles.panelHead)}>
                  <h2 {...stylex.props(styles.panelTitle)}>
                    Distribución de energía en tus sets
                  </h2>
                  <span {...stylex.props(styles.panelMeta)}>
                    Perfilado automático
                  </span>
                </div>

                <div {...stylex.props(styles.vibeRow)}>
                  <div {...stylex.props(styles.vibeTopLine)}>
                    <span {...stylex.props(styles.vibeName)}>
                      Melódico, profundo y envolvente
                    </span>
                    <strong {...stylex.props(styles.vibePct)}>62%</strong>
                  </div>
                  <div {...stylex.props(styles.vibeDesc)}>
                    Bases hipnóticas y progresivas que sostienen la pista sin
                    fatiga auditiva.
                  </div>
                  <div {...stylex.props(styles.vibeBarBg)}>
                    <div
                      {...stylex.props(styles.vibeBarFill)}
                      style={{ width: '62%', backgroundColor: '#14b8a6' }}
                    />
                  </div>
                </div>

                <div {...stylex.props(styles.vibeRow)}>
                  <div {...stylex.props(styles.vibeTopLine)}>
                    <span {...stylex.props(styles.vibeName)}>
                      Energía de hora pico
                    </span>
                    <strong {...stylex.props(styles.vibePct)}>24%</strong>
                  </div>
                  <div {...stylex.props(styles.vibeDesc)}>
                    Cortes con transientes marcados y sub-bajo firme para el
                    clímax del set.
                  </div>
                  <div {...stylex.props(styles.vibeBarBg)}>
                    <div
                      {...stylex.props(styles.vibeBarFill)}
                      style={{ width: '24%', backgroundColor: '#22c55e' }}
                    />
                  </div>
                </div>

                <div {...stylex.props(styles.vibeRow)}>
                  <div {...stylex.props(styles.vibeTopLine)}>
                    <span {...stylex.props(styles.vibeName)}>
                      Vocal y cierre melódico
                    </span>
                    <strong {...stylex.props(styles.vibePct)}>14%</strong>
                  </div>
                  <div {...stylex.props(styles.vibeDesc)}>
                    Temas con presencia vocal pensados para aperturas o cierres
                    memorables.
                  </div>
                  <div {...stylex.props(styles.vibeBarBg)}>
                    <div
                      {...stylex.props(styles.vibeBarFill)}
                      style={{ width: '14%', backgroundColor: '#f59e0b' }}
                    />
                  </div>
                </div>
              </div>
            ) : (
              <div {...stylex.props(styles.panelCard)}>
                <div {...stylex.props(styles.panelHead)}>
                  <h2 {...stylex.props(styles.panelTitle)}>
                    Matriz armónica Camelot 1A–12A (escala menor)
                  </h2>
                  <span {...stylex.props(styles.panelAccentMeta)}>
                    Dominantes: 8A (34%) · 9A (28%) · 10A (19%)
                  </span>
                </div>
                <div {...stylex.props(styles.camelotGrid)}>
                  {PRO_CAMELOT_KEYS_MINOR.map((k) => {
                    const isDominant = ['8A', '9A', '10A'].includes(k.code);
                    return (
                      <div
                        key={k.code}
                        {...stylex.props(
                          styles.camelotCell,
                          isDominant && styles.camelotCellActive,
                        )}
                      >
                        <span
                          {...stylex.props(
                            styles.camelotCode,
                            isDominant && styles.camelotCodeActive,
                          )}
                        >
                          {k.code}
                        </span>
                        <span {...stylex.props(styles.camelotNote)}>
                          {k.note}
                        </span>
                        <span {...stylex.props(styles.camelotPct)}>
                          {k.pct}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </section>

          <section {...stylex.props(styles.dnaRight)}>
            <h2 {...stylex.props(styles.panelTitle)}>
              {viewMode === 'simple'
                ? 'Reglas activas al ordenar y sugerir mezclas'
                : 'Invariantes de curaduría y motor DSP'}
            </h2>

            <div {...stylex.props(styles.rulesRack)}>
              <div {...stylex.props(styles.ruleRow)}>
                <div {...stylex.props(styles.ruleInfo)}>
                  <div {...stylex.props(styles.ruleTitle)}>
                    Evitar cruce de voces
                  </div>
                  <div {...stylex.props(styles.ruleDesc)}>
                    Enlaza la salida instrumental (Cue 4) con la entrada limpia
                    (Cue 1) del siguiente tema.
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setRuleVocalClash(!ruleVocalClash)}
                  {...stylex.props(
                    styles.toggleBtn,
                    ruleVocalClash && styles.toggleActive,
                  )}
                >
                  {ruleVocalClash ? 'Activo' : 'Inactivo'}
                </button>
              </div>

              <div {...stylex.props(styles.ruleRow)}>
                <div {...stylex.props(styles.ruleInfo)}>
                  <div {...stylex.props(styles.ruleTitle)}>
                    Continuidad de sub-bajo
                  </div>
                  <div {...stylex.props(styles.ruleDesc)}>
                    Mantiene presión sonora constante al encadenar bombos y
                    líneas graves.
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setRuleSubBass(!ruleSubBass)}
                  {...stylex.props(
                    styles.toggleBtn,
                    ruleSubBass && styles.toggleActive,
                  )}
                >
                  {ruleSubBass ? 'Activo' : 'Inactivo'}
                </button>
              </div>

              <div {...stylex.props(styles.ruleRow)}>
                <div {...stylex.props(styles.ruleInfo)}>
                  <div {...stylex.props(styles.ruleTitle)}>
                    Filtro forense de calidad de estudio
                  </div>
                  <div {...stylex.props(styles.ruleDesc)}>
                    Descarta falsos lossless o rips comprimidos con corte
                    espectral bajo.
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setRuleAudioQuality(!ruleAudioQuality)}
                  {...stylex.props(
                    styles.toggleBtn,
                    ruleAudioQuality && styles.toggleActive,
                  )}
                >
                  {ruleAudioQuality ? 'Activo' : 'Inactivo'}
                </button>
              </div>

              <div {...stylex.props(styles.ruleRow)}>
                <div {...stylex.props(styles.ruleInfo)}>
                  <div {...stylex.props(styles.ruleTitle)}>
                    Nodo colaborativo del enjambre P2P
                  </div>
                  <div {...stylex.props(styles.ruleDesc)}>
                    Mantiene activa la red comunitaria de masters verificados
                    entre cabinas HERA.
                  </div>
                </div>
                <span {...stylex.props(styles.toggleLocked)}>
                  Siempre activo
                </span>
              </div>
            </div>

            <div {...stylex.props(styles.summaryBox)}>
              <div {...stylex.props(styles.summaryHead)}>
                <span {...stylex.props(styles.summaryTitle)}>
                  Síntesis para el Copiloto de Cabina
                </span>
                <span {...stylex.props(styles.summaryStatus)}>
                  {' '}
                  Sincronizado
                </span>
              </div>
              <p {...stylex.props(styles.summaryBody)}>
                Al armar sets o puentes para{' '}
                <strong>{preferredSoftware}</strong>, HERA priorizará
                transiciones alrededor de <strong>{averageBpm}</strong>,
                enlazando el <strong>Cue 4 (Salida)</strong> con el{' '}
                <strong>Cue 1 (Entrada)</strong> en llaves vecinas para evitar
                choques armónicos y vocales.
              </p>
            </div>
          </section>
        </div>
      </div>
    </View>
  );
}

const styles = stylex.create({
  pageContainer: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    backgroundColor: '#0c0e12',
    color: '#edeff3',
    overflow: 'hidden',
  },
  headerBar: {
    paddingBlock: '14px',
    paddingInline: '20px',
    backgroundColor: '#12161d',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    flexWrap: 'wrap',
    rowGap: '10px',
    columnGap: '16px',
    flexShrink: 0,
  },
  titleBlock: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
  },
  titleRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  pageTitle: {
    margin: 0,
    fontSize: '15px',
    fontWeight: 700,
    color: '#edeff3',
  },
  subDesc: {
    margin: 0,
    fontSize: '12px',
    color: '#8e98a8',
  },
  switchesGroup: {
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    rowGap: '6px',
    columnGap: '6px',
  },
  deckLabel: {
    fontSize: '12px',
    color: '#8e98a8',
    marginRight: '4px',
  },
  swBtn: {
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
    transition: 'border-color 0.14s ease, color 0.14s ease',
  },
  swBtnActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    borderColor: 'rgba(20, 184, 166, 0.4)',
    color: '#2dd4bf',
    fontWeight: 600,
  },
  dnaWorkspace: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    display: 'grid',
    gridTemplateColumns: '1.15fr 0.95fr',
    columnGap: '18px',
    padding: '20px',
    overflowY: 'auto',
  },
  dnaLeft: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '16px',
    minWidth: 0,
  },
  dnaRight: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '14px',
    minWidth: 0,
  },
  telemetryRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '12px',
  },
  telemetryCard: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '4px',
  },
  tVal: {
    fontSize: '18px',
    fontWeight: 700,
    color: '#edeff3',
    fontVariantNumeric: 'tabular-nums',
    lineHeight: 1.2,
  },
  tValEmerald: {
    color: '#2dd4bf',
  },
  tName: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
  },
  tSub: {
    fontSize: '11.5px',
    color: '#8e98a8',
    lineHeight: 1.35,
  },
  panelCard: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '16px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '14px',
  },
  panelHead: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  panelTitle: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  panelMeta: {
    fontSize: '11.5px',
    color: '#8e98a8',
  },
  panelAccentMeta: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
  },
  vibeRow: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
  },
  vibeTopLine: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    fontSize: '12.5px',
  },
  vibeName: {
    fontWeight: 600,
    color: '#edeff3',
  },
  vibePct: {
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
  },
  vibeDesc: {
    fontSize: '12px',
    color: '#8e98a8',
    lineHeight: 1.4,
  },
  vibeBarBg: {
    height: '6px',
    width: '100%',
    backgroundColor: '#0c0e12',
    borderRadius: '999px',
    overflow: 'hidden',
  },
  vibeBarFill: {
    height: '100%',
    borderRadius: '999px',
  },
  camelotGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(6, 1fr)',
    rowGap: '8px',
    columnGap: '8px',
  },
  camelotCell: {
    paddingBlock: '10px',
    paddingInline: '8px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
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
    borderColor: 'rgba(20, 184, 166, 0.36)',
  },
  camelotCode: {
    fontWeight: 700,
    fontSize: '12.5px',
    color: '#edeff3',
  },
  camelotCodeActive: {
    color: '#2dd4bf',
  },
  camelotNote: {
    fontSize: '10.5px',
    color: '#8e98a8',
  },
  camelotPct: {
    fontSize: '10.5px',
    color: '#8e98a8',
  },
  rulesRack: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  ruleRow: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    paddingBlock: '11px',
    paddingInline: '14px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '14px',
  },
  ruleInfo: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
    minWidth: 0,
  },
  ruleTitle: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#edeff3',
  },
  ruleDesc: {
    fontSize: '12px',
    color: '#8e98a8',
    lineHeight: 1.4,
  },
  toggleBtn: {
    flexShrink: 0,
    paddingBlock: '5px',
    paddingInline: '11px',
    borderRadius: '6px',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
  },
  toggleActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    borderColor: 'rgba(20, 184, 166, 0.38)',
    color: '#2dd4bf',
  },
  toggleLocked: {
    flexShrink: 0,
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    fontSize: '11.5px',
    fontWeight: 600,
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#2dd4bf',
  },
  summaryBox: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    borderRadius: '8px',
    padding: '15px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  summaryHead: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  summaryTitle: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#edeff3',
  },
  summaryStatus: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#2dd4bf',
  },
  summaryBody: {
    margin: 0,
    fontSize: '12.5px',
    lineHeight: 1.55,
    color: '#8e98a8',
  },
});

import * as stylex from '@stylexjs/stylex';
import { useState } from 'react';
import Icon from './Icon';

type Props = {
  isOpen: boolean;
  onClose: () => void;
};

export default function OnboardingWizardModal({ isOpen, onClose }: Props) {
  const [step, setStep] = useState<1 | 2 | 3>(1);

  if (!isOpen) return null;

  return (
    <div {...stylex.props(styles.overlay)} role="dialog" aria-modal="true" aria-labelledby="onboarding-title">
      <div {...stylex.props(styles.card)}>
        {/* Header */}
        <div {...stylex.props(styles.header)}>
          <div {...stylex.props(styles.badgeRow)}>
            <span {...stylex.props(styles.betaBadge)}>HERA v0.6.0 · BETA</span>
            <span {...stylex.props(styles.stepCounter)}>Paso {step} de 3</span>
          </div>
          <button
            type="button"
            onClick={onClose}
            {...stylex.props(styles.closeBtn)}
            aria-label="Cerrar asistente"
          >
            <Icon name="x" size={16} />
          </button>
        </div>

        {/* Content per Step */}
        <div {...stylex.props(styles.body)}>
          {step === 1 && (
            <div {...stylex.props(styles.stepContent)}>
              <div {...stylex.props(styles.iconWrapper)}>
                <Icon name="sparkles" size={28} color="#14b8a6" />
              </div>
              <h2 id="onboarding-title" {...stylex.props(styles.title)}>
                Bienvenido a HERA Studio BETA
              </h2>
              <p {...stylex.props(styles.description)}>
                HERA es tu motor de curación y DJ Copilot 100% en Rust nativo (sin Python).
                Diseñado para operar en cabina con baja latencia y alta fidelidad sonora.
              </p>
              <div {...stylex.props(styles.alertBox)}>
                <strong {...stylex.props(styles.alertHeading)}>
                  Nota del Release BETA:
                </strong>
                <p {...stylex.props(styles.alertText)}>
                  Estamos calibrando el motor con DJs en vivo. Tu música física nunca se altera,
                  no se renombra ni se borra del disco. Todas las clasificaciones son puramente lógicas.
                </p>
              </div>
            </div>
          )}

          {step === 2 && (
            <div {...stylex.props(styles.stepContent)}>
              <div {...stylex.props(styles.iconWrapper)}>
                <Icon name="disc" size={28} color="#38bdf8" />
              </div>
              <h2 id="onboarding-title" {...stylex.props(styles.title)}>
                Escaneo Inteligente & Criterio de Cabina
              </h2>
              <p {...stylex.props(styles.description)}>
                El nuevo escáner térmico procesa tu música con total respeto por tu CPU:
              </p>
              <ul {...stylex.props(styles.featureList)}>
                <li {...stylex.props(styles.featureItem)}>
                  <strong>Caché Ultrarrápido:</strong> Si usas VirtualDJ o Rekordbox, lee sus bases de datos en &lt; 3 segundos sin gastar procesador.
                </li>
                <li {...stylex.props(styles.featureItem)}>
                  <strong>Protección Térmica:</strong> Escaneo en lotes de 8 pistas con pausas cooperativas para evitar que tu laptop se caliente.
                </li>
                <li {...stylex.props(styles.featureItem)}>
                  <strong>Filtro de Mixes (&gt; 10 min):</strong> Las sesiones largas y sets se aíslan en la pestaña de mixes sin saturar tus crates rápidos.
                </li>
                <li {...stylex.props(styles.featureItem)}>
                  <strong>Detección de Saltos de CD:</strong> Identifica pistas dañadas o rayadas antes de que suenen en vivo.
                </li>
              </ul>
            </div>
          )}

          {step === 3 && (
            <div {...stylex.props(styles.stepContent)}>
              <div {...stylex.props(styles.iconWrapper)}>
                <Icon name="bridge" size={28} color="#22c55e" />
              </div>
              <h2 id="onboarding-title" {...stylex.props(styles.title)}>
                Integración de Cabina en Vivo
              </h2>
              <p {...stylex.props(styles.description)}>
                Conecta tu sesión en directo con tus herramientas favoritas:
              </p>
              <div {...stylex.props(styles.cardsGrid)}>
                <div {...stylex.props(styles.miniCard)}>
                  <Icon name="crateList" size={20} color="#14b8a6" />
                  <h4 {...stylex.props(styles.miniCardTitle)}>VirtualDJ Live</h4>
                  <p {...stylex.props(styles.miniCardText)}>
                    Envía crates directamente a tu carpeta VirtualDJ con 4 Hot Cues acústicos listos.
                  </p>
                </div>
                <div {...stylex.props(styles.miniCard)}>
                  <Icon name="sliders" size={20} color="#f59e0b" />
                  <h4 {...stylex.props(styles.miniCardTitle)}>Modo Mini</h4>
                  <p {...stylex.props(styles.miniCardText)}>
                    Presiona Ctrl+Shift+M para una barra compacta tipo HUD mientras mezclas en pantalla completa.
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer Buttons */}
        <div {...stylex.props(styles.footer)}>
          {step > 1 ? (
            <button
              type="button"
              onClick={() => setStep((s) => (s - 1) as 1 | 2)}
              {...stylex.props(styles.secondaryBtn)}
            >
              Anterior
            </button>
          ) : (
            <div />
          )}

          {step < 3 ? (
            <button
              type="button"
              onClick={() => setStep((s) => (s + 1) as 2 | 3)}
              {...stylex.props(styles.primaryBtn)}
            >
              Siguiente
            </button>
          ) : (
            <button
              type="button"
              onClick={onClose}
              {...stylex.props(styles.primaryBtn)}
            >
              ¡Comenzar a Mezclar!
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

const styles = stylex.create({
  overlay: {
    position: 'fixed',
    inset: 0,
    backgroundColor: 'rgba(5, 7, 10, 0.78)',
    backdropFilter: 'blur(6px)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 9999,
    padding: '16px',
  },
  card: {
    width: '100%',
    maxWidth: '540px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '12px',
    boxShadow: '0 20px 48px rgba(0, 0, 0, 0.65)',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '16px 20px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#1a202c',
  },
  badgeRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
  },
  betaBadge: {
    fontSize: '11px',
    fontWeight: 700,
    letterSpacing: '0.06em',
    color: '#0d9488',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.3)',
    borderRadius: '999px',
    padding: '2px 8px',
  },
  stepCounter: {
    fontSize: '12px',
    color: '#8b949e',
  },
  closeBtn: {
    background: 'none',
    border: 'none',
    color: '#8b949e',
    cursor: 'pointer',
    padding: '4px',
    borderRadius: '6px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  body: {
    padding: '24px 20px',
    minHeight: '260px',
  },
  stepContent: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    textAlign: 'center',
  },
  iconWrapper: {
    width: '52px',
    height: '52px',
    borderRadius: '50%',
    backgroundColor: 'rgba(255, 255, 255, 0.04)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    marginBottom: '14px',
  },
  title: {
    fontSize: '17px',
    fontWeight: 700,
    color: '#f0f3f6',
    margin: '0 0 8px 0',
  },
  description: {
    fontSize: '13px',
    color: '#9ba3af',
    lineHeight: 1.5,
    margin: '0 0 16px 0',
    maxWidth: '460px',
  },
  alertBox: {
    backgroundColor: 'rgba(20, 184, 166, 0.06)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.2)',
    borderRadius: '8px',
    padding: '12px 14px',
    textAlign: 'left',
    width: '100%',
  },
  alertHeading: {
    fontSize: '12px',
    color: '#14b8a6',
    display: 'block',
    marginBottom: '4px',
  },
  alertText: {
    fontSize: '12px',
    color: '#8b949e',
    lineHeight: 1.4,
    margin: 0,
  },
  featureList: {
    textAlign: 'left',
    fontSize: '12.5px',
    color: '#9ba3af',
    lineHeight: 1.5,
    margin: 0,
    paddingLeft: '20px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
    width: '100%',
  },
  featureItem: {
    color: '#c9d1d9',
  },
  cardsGrid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '12px',
    width: '100%',
    marginTop: '6px',
    textAlign: 'left',
  },
  miniCard: {
    backgroundColor: 'rgba(255, 255, 255, 0.02)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '12px',
  },
  miniCardTitle: {
    fontSize: '13px',
    fontWeight: 600,
    color: '#f0f3f6',
    margin: '8px 0 4px 0',
  },
  miniCardText: {
    fontSize: '11.5px',
    color: '#8b949e',
    margin: 0,
    lineHeight: 1.35,
  },
  footer: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '14px 20px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1a202c',
    backgroundColor: '#090b0e',
  },
  secondaryBtn: {
    backgroundColor: 'transparent',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#2d3748',
    color: '#c9d1d9',
    fontSize: '12.5px',
    fontWeight: 500,
    padding: '6px 14px',
    borderRadius: '6px',
    cursor: 'pointer',
  },
  primaryBtn: {
    backgroundColor: '#14b8a6',
    border: 'none',
    color: '#041c18',
    fontSize: '12.5px',
    fontWeight: 600,
    padding: '6px 16px',
    borderRadius: '6px',
    cursor: 'pointer',
  },
});

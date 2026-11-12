import * as stylex from '@stylexjs/stylex';
import { useNavigate } from '@tanstack/react-router';

import Icon from './Icon';
import View from '../elements/View';
import { useFeatureFlags, type FeatureFlags } from '../lib/feature-flags';

type UnderConstructionViewProps = {
  title: string;
  description: string;
  flagKey?: keyof FeatureFlags;
};

export default function UnderConstructionView({
  title,
  description,
  flagKey,
}: UnderConstructionViewProps) {
  const navigate = useNavigate();
  const setFlag = useFeatureFlags((s) => s.setFlag);

  return (
    <View aria-label={title}>
      <div {...stylex.props(styles.viewport)}>
        <div {...stylex.props(styles.card)}>
          <div {...stylex.props(styles.headerRow)}>
            <div {...stylex.props(styles.iconFrame)}>
              <Icon name="sliders" size={20} color="#2dd4bf" />
            </div>
            <span {...stylex.props(styles.labsBadge)}>HERA LABS · EN DESARROLLO</span>
          </div>

          <h1 {...stylex.props(styles.title)}>{title}</h1>

          <p {...stylex.props(styles.body)}>{description}</p>

          <div {...stylex.props(styles.noticeBox)}>
            <span {...stylex.props(styles.noticeText)}>
              ⚙️ Esta función se encuentra actualmente en calibración matemática de DSP y pruebas de laboratorio. Estará disponible en una próxima actualización de HERA.
            </span>
          </div>

          <div {...stylex.props(styles.actionsRow)}>
            <button
              type="button"
              onClick={() => void navigate({ to: '/library' })}
              {...stylex.props(styles.primaryBtn)}
            >
              <Icon name="disc" size={16} />
              <span>Volver a la Biblioteca</span>
            </button>

            {flagKey ? (
              <button
                type="button"
                onClick={() => setFlag(flagKey, true)}
                title="Activar vista preliminar para pruebas internas"
                {...stylex.props(styles.devBtn)}
              >
                <span>Habilitar para pruebas de Lab</span>
              </button>
            ) : null}
          </div>
        </div>
      </div>
    </View>
  );
}

const styles = stylex.create({
  viewport: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    height: '100%',
    width: '100%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '28px',
    backgroundColor: '#0c0e12',
  },
  card: {
    maxWidth: '560px',
    width: '100%',
    paddingBlock: '32px',
    paddingInline: '32px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-start',
    rowGap: '16px',
    boxShadow: '0 8px 24px rgba(0, 0, 0, 0.4)',
  },
  headerRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
  },
  iconFrame: {
    width: '40px',
    height: '40px',
    borderRadius: '8px',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.3)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  labsBadge: {
    fontSize: '10.5px',
    fontWeight: 700,
    letterSpacing: '0.08em',
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.25)',
    paddingBlock: '3px',
    paddingInline: '8px',
    borderRadius: '6px',
  },
  title: {
    margin: 0,
    fontSize: '20px',
    fontWeight: 600,
    letterSpacing: '-0.02em',
    color: '#edeff3',
  },
  body: {
    margin: 0,
    fontSize: '13px',
    lineHeight: 1.6,
    color: '#9ba5b5',
  },
  noticeBox: {
    width: '100%',
    paddingBlock: '10px',
    paddingInline: '14px',
    backgroundColor: '#161c26',
    borderLeftWidth: '3px',
    borderLeftStyle: 'solid',
    borderLeftColor: '#14b8a6',
    borderRadius: '4px',
  },
  noticeText: {
    fontSize: '12px',
    color: '#cbd2dc',
    lineHeight: 1.5,
  },
  actionsRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
    marginTop: '6px',
    flexWrap: 'wrap',
    rowGap: '8px',
  },
  primaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '8px',
    paddingInline: '16px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontSize: '13px',
    fontWeight: 600,
    cursor: 'pointer',
    transition: 'opacity 0.15s ease',
  },
  devBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '8px',
    paddingInline: '14px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    backgroundColor: '#171c24',
    color: '#8e98a8',
    fontSize: '12px',
    fontWeight: 500,
    cursor: 'pointer',
    transition: 'color 0.15s ease, border-color 0.15s ease',
  },
});

import * as stylex from '@stylexjs/stylex';

import type { CommanderAuthorityMode } from '../../lib/bridge-hera';

export type TransitionAssistantPopoverProps = {
  connectionScope: 'cloud' | 'offline';
  commanderMode: CommanderAuthorityMode;
  showAdvancedUrl: boolean;
  customBaseUrl: string;
  onConnectionScopeChange: (scope: 'cloud' | 'offline') => void;
  onCommanderModeChange: (mode: CommanderAuthorityMode) => void;
  onToggleAdvancedUrl: () => void;
  onCustomBaseUrlChange: (url: string) => void;
};

export default function TransitionAssistantPopover({
  connectionScope,
  commanderMode,
  showAdvancedUrl,
  customBaseUrl,
  onConnectionScopeChange,
  onCommanderModeChange,
  onToggleAdvancedUrl,
  onCustomBaseUrlChange,
}: TransitionAssistantPopoverProps) {
  return (
    <dialog
      open
      aria-label="Asistente"
      {...stylex.props(styles.assistantPopover)}
    >
      <div {...stylex.props(styles.popoverHead)}>
        <div {...stylex.props(styles.popoverTitleGroup)}>
          <span {...stylex.props(styles.popoverTitle)}>Asistente</span>
          <span {...stylex.props(styles.readyDot)} />
        </div>
        <span {...stylex.props(styles.popoverReadyText)}>Listo</span>
      </div>

      <div {...stylex.props(styles.popoverRow)}>
        <span {...stylex.props(styles.popoverLabel)}>Conexión</span>
        <div {...stylex.props(styles.popoverSegment)}>
          <button
            type="button"
            onClick={() => onConnectionScopeChange('cloud')}
            {...stylex.props(
              styles.popSegBtn,
              connectionScope === 'cloud' && styles.popSegBtnTeal,
            )}
          >
            Nube
          </button>
          <button
            type="button"
            onClick={() => onConnectionScopeChange('offline')}
            {...stylex.props(
              styles.popSegBtn,
              connectionScope === 'offline' && styles.popSegBtnTeal,
            )}
          >
            Sin internet
          </button>
        </div>
      </div>

      <div {...stylex.props(styles.popoverRow, styles.popoverRowDivider)}>
        <span {...stylex.props(styles.popoverLabel)}>Modo</span>
        <div {...stylex.props(styles.popoverSegment)}>
          <button
            type="button"
            onClick={() => onCommanderModeChange('executive_boss')}
            {...stylex.props(
              styles.popSegBtn,
              commanderMode === 'executive_boss' && styles.popSegBtnNeutral,
            )}
          >
            Sugerir
          </button>
          <button
            type="button"
            onClick={() => onCommanderModeChange('autonomous_operator')}
            {...stylex.props(
              styles.popSegBtn,
              commanderMode === 'autonomous_operator' &&
                styles.popSegBtnNeutral,
            )}
          >
            Automático
          </button>
        </div>
      </div>

      {showAdvancedUrl && (
        <div {...stylex.props(styles.advancedBox)}>
          <input
            type="text"
            value={customBaseUrl}
            onChange={(e) => onCustomBaseUrlChange(e.target.value)}
            placeholder="http://127.0.0.1:11434/v1"
            aria-label="Servidor local"
            {...stylex.props(styles.advancedInput)}
          />
        </div>
      )}

      <div {...stylex.props(styles.popoverFooter)}>
        <button
          type="button"
          onClick={onToggleAdvancedUrl}
          {...stylex.props(styles.advancedLinkBtn)}
        >
          {showAdvancedUrl ? 'Ocultar servidor' : 'Conexión avanzada…'}
        </button>
      </div>
    </dialog>
  );
}

const styles = stylex.create({
  assistantPopover: {
    position: 'absolute',
    left: 'auto',
    right: 0,
    bottom: '46px',
    width: '296px',
    margin: 0,
    padding: 0,
    color: '#edeff3',
    borderRadius: '12px',
    backgroundColor: '#151a23',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#283140',
    boxShadow: '0 16px 40px rgba(0, 0, 0, 0.55)',
    zIndex: 50,
    overflow: 'hidden',
  },
  popoverHead: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '12px',
    paddingInline: '16px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#202836',
  },
  popoverTitleGroup: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  popoverTitle: {
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  readyDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
  },
  popoverReadyText: {
    fontSize: '12px',
    color: '#8e98a8',
  },
  popoverRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '10px',
    paddingBlock: '11px',
    paddingInline: '16px',
  },
  popoverRowDivider: {
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#202836',
  },
  popoverLabel: {
    fontSize: '12.5px',
    color: '#edeff3',
    fontWeight: 500,
  },
  popoverSegment: {
    display: 'inline-flex',
    alignItems: 'center',
    padding: '2px',
    borderRadius: '999px',
    backgroundColor: '#0f131a',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#242c3b',
  },
  popSegBtn: {
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '999px',
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  popSegBtnTeal: {
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontWeight: 600,
  },
  popSegBtnNeutral: {
    backgroundColor: '#252e3d',
    color: '#edeff3',
    fontWeight: 600,
  },
  advancedBox: {
    paddingTop: '4px',
    paddingBottom: '10px',
    paddingInline: '16px',
  },
  advancedInput: {
    width: '100%',
    paddingBlock: '6px',
    paddingInline: '9px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    color: '#edeff3',
    fontSize: '11.5px',
  },
  popoverFooter: {
    paddingBlock: '9px',
    paddingInline: '16px',
    backgroundColor: '#11161e',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#202836',
    textAlign: 'center',
  },
  advancedLinkBtn: {
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    fontSize: '11.5px',
    cursor: 'pointer',
  },
});

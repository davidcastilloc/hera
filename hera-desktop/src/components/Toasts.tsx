import { Toast } from '@base-ui/react/toast';
import * as stylex from '@stylexjs/stylex';

import toastManager from '../lib/toast-manager';

export default function Toasts() {
  return (
    <Toast.Provider toastManager={toastManager} timeout={3200} limit={1}>
      <Toast.Portal>
        <Toast.Viewport {...stylex.props(styles.viewport)}>
          <ToastList />
        </Toast.Viewport>
      </Toast.Portal>
    </Toast.Provider>
  );
}

function ToastList() {
  const { toasts } = Toast.useToastManager();
  const actionableToasts = toasts.filter(
    (toast) => toast.type === 'warning' || toast.type === 'danger',
  );

  return actionableToasts.map((toast) => (
    <Toast.Root
      key={toast.id}
      toast={toast}
      onClick={() => toastManager.close(toast.id)}
      {...stylex.props(
        styles.toast,
        toast.type === 'danger' ? styles.danger : styles.warning,
      )}
    >
      {toast.title}
    </Toast.Root>
  ));
}

const styles = stylex.create({
  viewport: {
    position: 'fixed',
    bottom: '16px',
    right: '16px',
    width: '280px',
    zIndex: 1000,
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
    pointerEvents: 'none',
  },
  toast: {
    borderStyle: 'solid',
    borderWidth: '1px',
    backgroundColor: '#161b24',
    borderColor: '#2a3342',
    borderRadius: '8px',
    boxShadow: '0 8px 24px rgba(0, 0, 0, 0.42)',
    color: '#edeff3',
    paddingBlock: '10px',
    paddingInline: '12px',
    fontSize: '12px',
    lineHeight: 1.4,
    cursor: 'pointer',
    pointerEvents: 'auto',
  },
  warning: {
    borderColor: 'rgba(245, 158, 11, 0.45)',
  },
  danger: {
    borderColor: 'rgba(239, 68, 68, 0.5)',
  },
});

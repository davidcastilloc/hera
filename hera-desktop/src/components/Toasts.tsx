import { Toast } from '@base-ui/react/toast';
import * as stylex from '@stylexjs/stylex';

import toastManager from '../lib/toast-manager';

export default function Toasts() {
  return (
    <Toast.Provider toastManager={toastManager} timeout={2800} limit={5}>
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
  return toasts.map((toast) => (
    <Toast.Root
      key={toast.id}
      toast={toast}
      onClick={() => toastManager.close(toast.id)}
      {...stylex.props(
        styles.toast,
        typeVariantsStyles[toast.type as keyof typeof typeVariantsStyles] ??
          typeVariantsStyles['info'],
      )}
    >
      {toast.title}
    </Toast.Root>
  ));
}

const styles = stylex.create({
  viewport: {
    position: 'fixed',
    top: '56px',
    right: '12px',
    width: '260px',
    zIndex: 1000,
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
    columnGap: '8px',
    pointerEvents: 'none',
  },
  toast: {
    borderStyle: 'solid',
    borderWidth: '1px',
    borderLeftWidth: '5px',
    backgroundColor: 'var(--toasts-bg)',
    borderTopColor: 'var(--border-color)',
    borderRightColor: 'var(--border-color)',
    borderBottomColor: 'var(--border-color)',
    borderRadius: 'var(--border-radius)',
    color: 'var(--text)',
    paddingBlock: '10px',
    paddingInline: '12px',
    fontSize: '12px',
    cursor: 'pointer',
    pointerEvents: 'auto',
  },
});

const typeVariantsStyles = stylex.create({
  success: {
    borderLeftColor: 'var(--success-color)',
  },
  info: {
    borderLeftColor: 'var(--info-color)',
  },
  warning: {
    borderLeftColor: 'var(--warning-color)',
  },
  danger: {
    borderLeftColor: 'var(--danger-color)',
  },
});

export type ToastType = keyof typeof typeVariantsStyles;

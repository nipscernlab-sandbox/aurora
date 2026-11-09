// Os avisos rápidos, no canto inferior direito.

import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from 'lucide-react';

import { useToasts } from '../state/toasts';

const ICONS = { error: CircleAlert, success: CircleCheck, info: Info, warning: TriangleAlert };

export function Toasts() {
  const toasts = useToasts((s) => s.toasts);
  return (
    <div className="toasts" aria-live="polite">
      {toasts.map((toast) => {
        const Icon = ICONS[toast.kind];
        return (
          <div key={toast.id} className={`toast toast--${toast.kind}`} role="status">
            <Icon size={15} className="toast__icon" />
            <div className="toast__text">
              <div className="toast__title">{toast.title}</div>
              {toast.detail && <div className="toast__detail">{toast.detail}</div>}
              {toast.action && (
                <button
                  type="button"
                  className="btn btn--sm toast__action"
                  onClick={() => {
                    useToasts.getState().dismiss(toast.id);
                    toast.action!.run();
                  }}
                >
                  {toast.action.label}
                </button>
              )}
            </div>
            <button type="button" className="icon-btn icon-btn--small" onClick={() => useToasts.getState().dismiss(toast.id)}>
              <X size={13} />
            </button>
          </div>
        );
      })}
    </div>
  );
}

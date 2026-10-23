// DS: toasts.html · .toast-region, .toast, .toast--error/--info/--warning, .toast__title, .toast__desc, .toast__actions
// Erros ficam até o usuário fechar; confirmações somem em 6 s.
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { IconButton } from "./Button";
import { Icon } from "./Icon";

type ToastKind = "success" | "info" | "warning" | "error";

interface ToastInput {
  kind: ToastKind;
  title: string;
  description?: string;
  action?: { label: string; onClick: () => void };
}

interface ToastItem extends ToastInput {
  id: number;
}

const ICON: Record<ToastKind, string> = { success: "success", info: "info", warning: "warning", error: "error" };

const ToastContext = createContext<(toast: ToastInput) => void>(() => undefined);

export const useToast = () => useContext(ToastContext);

let nextId = 1;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const dismiss = useCallback((id: number) => setToasts((all) => all.filter((t) => t.id !== id)), []);

  const show = useCallback((toast: ToastInput) => {
    const id = nextId++;
    setToasts((all) => [...all.slice(-2), { ...toast, id }]);
    if (toast.kind !== "error") setTimeout(() => dismiss(id), 6000);
  }, [dismiss]);

  const value = useMemo(() => show, [show]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="toast-region" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={["toast", t.kind !== "success" && `toast--${t.kind}`].filter(Boolean).join(" ")} role={t.kind === "error" ? "alert" : "status"}>
            <Icon name={ICON[t.kind]} />
            <div>
              <p className="toast__title">{t.title}</p>
              {t.description && <p className="toast__desc">{t.description}</p>}
              {t.action && (
                <div className="toast__actions">
                  <button type="button" onClick={() => { dismiss(t.id); t.action!.onClick(); }}>{t.action.label}</button>
                </div>
              )}
            </div>
            <IconButton icon="x" label="Fechar" compact onClick={() => dismiss(t.id)} />
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

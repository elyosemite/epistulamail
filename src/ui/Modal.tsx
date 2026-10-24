// DS: modais.html · .blanket, .modal, .modal--warning, .modal__foot--split, .modal__head, .modal__title, .modal__body, .modal__foot
// Foco preso no modal; Esc fecha; o foco volta para onde estava ao fechar.
import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from "react";
import { Icon } from "./Icon";

interface ModalProps {
  title: string;
  icon?: string;
  appearance?: "default" | "warning";
  /** Seletor do elemento que recebe o foco ao abrir (padrão: o primeiro focável). */
  initialFocus?: string;
  /** Primeiro botão do rodapé à esquerda, os demais à direita (`.modal__foot--split`). */
  splitFooter?: boolean;
  onClose: () => void;
  children: ReactNode;
  footer: ReactNode;
}

const FOCUSABLE = "button:not(:disabled), input:not(:disabled), [tabindex]:not([tabindex='-1'])";

export function Modal({ title, icon, appearance = "default", initialFocus, splitFooter, onClose, children, footer }: ModalProps) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const target = (initialFocus && ref.current?.querySelector<HTMLElement>(initialFocus)) || ref.current?.querySelector<HTMLElement>(FOCUSABLE);
    target?.focus();
    return () => previous?.focus?.();
  }, [initialFocus]);

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key !== "Tab") return;
    const items = Array.from(ref.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? []);
    if (items.length === 0) return;
    const i = items.indexOf(document.activeElement as HTMLElement);
    e.preventDefault();
    items[(i + (e.shiftKey ? -1 : 1) + items.length) % items.length].focus();
  };

  const cls = ["modal", appearance === "warning" && "modal--warning"].filter(Boolean).join(" ");
  return (
    <div className="blanket" onKeyDown={onKeyDown}>
      <div ref={ref} className={cls} role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div className="modal__head">
          {icon && <Icon name={icon} />}
          <h2 className="modal__title" id={titleId}>{title}</h2>
        </div>
        <div className="modal__body">{children}</div>
        <div className={splitFooter ? "modal__foot modal__foot--split" : "modal__foot"}>{footer}</div>
      </div>
    </div>
  );
}

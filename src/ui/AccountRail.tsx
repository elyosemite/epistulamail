// DS: sidebar.html "Coluna de contas" · .account-rail, __list, __item ([aria-current], --signed-out), __name, __badge
//     (.badge.badge--primary), __alert, __sep, __add
// DS: espacamento.html · .app-shell__rail (área da casca .app-shell--with-rail)
import { forwardRef, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { Icon } from "./Icon";

export function AccountRail({ children, onAdd }: { children: ReactNode; onAdd: () => void }) {
  return (
    <nav className="account-rail app-shell__rail" aria-label="Contas">
      <div className="account-rail__list" role="list">{children}</div>
      <div className="account-rail__sep" role="separator" />
      <button className="account-rail__add" type="button" aria-label="Adicionar conta" data-tip="Adicionar conta" onClick={onAdd}>
        <Icon name="plus" />
      </button>
    </nav>
  );
}

interface AccountRailItemProps {
  name: string;
  initial: string;
  /** Classe de cor do avatar (`avatar--purple`) ou `null` para o azul padrão. */
  colorClass: string | null;
  badge: string | null;
  state: "normal" | "alert" | "signed-out";
  current: boolean;
  label: string;
  tip: string;
  onSelect: () => void;
  onMenu: (anchor: { x: number; y: number }) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

export const AccountRailItem = forwardRef<HTMLButtonElement, AccountRailItemProps>(function AccountRailItem(
  { name, initial, colorClass, badge, state, current, label, tip, onSelect, onMenu, onKeyDown },
  ref,
) {
  const cls = ["account-rail__item", state === "signed-out" && "account-rail__item--signed-out"].filter(Boolean).join(" ");
  const avatar = ["avatar avatar--lg", colorClass].filter(Boolean).join(" ");
  const openMenu = (event: MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    onMenu({ x: event.clientX, y: event.clientY });
  };
  const keyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
      event.preventDefault();
      const r = event.currentTarget.getBoundingClientRect();
      onMenu({ x: r.right, y: r.top });
      return;
    }
    onKeyDown?.(event);
  };
  return (
    <div role="listitem">
      <button
        ref={ref}
        type="button"
        className={cls}
        aria-current={current ? "true" : undefined}
        aria-label={label}
        aria-haspopup="menu"
        data-tip={tip}
        onClick={onSelect}
        onContextMenu={openMenu}
        onKeyDown={keyDown}
      >
        <span className={avatar} aria-hidden="true">{initial}</span>
        {badge && state === "normal" && <span className="badge badge--primary account-rail__badge" aria-hidden="true">{badge}</span>}
        {state === "alert" && (
          <span className="account-rail__alert" aria-hidden="true"><Icon name="warning" /></span>
        )}
        <span className="account-rail__name" aria-hidden="true">{name}</span>
      </button>
    </div>
  );
});

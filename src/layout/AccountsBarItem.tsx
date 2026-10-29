// DS: contas.html "Coluna de contas" · .account-rail__item ([aria-current], --signed-out), __name, __badge
//     (.badge.badge--primary), __alert; avatar.html · .avatar--lg com .avatar__img (foto da conta)
import { forwardRef, type KeyboardEvent, type MouseEvent } from "react";
import { AccountAvatar } from "../ui/AccountAvatar";
import { Icon } from "../ui/Icon";

interface AccountsBarItemProps {
  name: string;
  initial: string;
  /** Classe de cor do avatar (`avatar--purple`) ou `null` para o azul padrão. */
  colorClass: string | null;
  photo: string | null;
  /** Arquivo local (US15): avatar neutro com o ícone (L71). */
  archive?: boolean;
  badge: string | null;
  state: "normal" | "alert" | "signed-out";
  current: boolean;
  label: string;
  tip: string;
  onSelect: () => void;
  onMenu: (anchor: { x: number; y: number }) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

/** Uma conta na coluna de contas. */
export const AccountsBarItem = forwardRef<HTMLButtonElement, AccountsBarItemProps>(function AccountsBarItem(
  { name, initial, colorClass, photo, archive, badge, state, current, label, tip, onSelect, onMenu, onKeyDown },
  ref,
) {
  const cls = ["account-rail__item", state === "signed-out" && "account-rail__item--signed-out"].filter(Boolean).join(" ");
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
        <AccountAvatar initial={initial} colorClass={colorClass} photo={photo} size="lg" archive={archive} />
        {badge && state === "normal" && <span className="badge badge--primary account-rail__badge" aria-hidden="true">{badge}</span>}
        {state === "alert" && (
          <span className="account-rail__alert" aria-hidden="true"><Icon name="warning" /></span>
        )}
        <span className="account-rail__name" aria-hidden="true">{name}</span>
      </button>
    </div>
  );
});

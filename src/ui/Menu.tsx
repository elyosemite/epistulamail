// DS: menus.html · .menu (--wide), .menu__heading, .menu__sep, .menu__search, .menu__form, .menu__empty, .menu-item,
//     .menu-item__text, .menu-item__meta, .menu-item.is-selected, .menu-item--danger
// Camada fixa ancorada num ponto ou num botão; setas navegam entre itens e campos, Esc e clique fora fecham.
import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { Icon } from "./Icon";

const MARGIN = 8;
// Setas percorrem itens, campos, botões das linhas do menu da conta e a opção escolhida de cada .segmented.
// No painel do sino (US8), também as notificações e os botões do topo e do rodapé.
const NAVIGABLE = ".menu-item:not(:disabled):not([hidden]), .menu input, .account-menu__line .btn:not(:disabled), .menu .segmented [aria-checked=\"true\"], .notice, .notice-panel .btn";

export type Anchor = { x: number; y: number };

/** Ponto logo abaixo do elemento, alinhado à esquerda dele. */
export function anchorBelow(el: Element): Anchor {
  const r = el.getBoundingClientRect();
  return { x: r.left, y: r.bottom + 4 };
}

interface MenuPanelProps {
  label: string;
  anchor: Anchor;
  wide?: boolean;
  /** Classe extra (ex.: `account-menu`). */
  className?: string;
  /** `dialog` para um painel que não é uma lista de comandos (o do sino). */
  role?: "menu" | "dialog";
  onClose: () => void;
  children: ReactNode;
}

export function MenuPanel({ label, anchor, wide, className, role = "menu", onClose, children }: MenuPanelProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState(anchor);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;

  // Mantém o menu dentro da janela.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setPosition({
      x: Math.max(MARGIN, Math.min(anchor.x, innerWidth - r.width - MARGIN)),
      y: Math.max(MARGIN, Math.min(anchor.y, innerHeight - r.height - MARGIN)),
    });
  }, [anchor]);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.querySelector<HTMLElement>(NAVIGABLE)?.focus();
    const onPointer = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) closeRef.current();
    };
    document.addEventListener("pointerdown", onPointer);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      previous?.focus?.();
    };
  }, []);

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const items = Array.from(ref.current?.querySelectorAll<HTMLElement>(NAVIGABLE) ?? []).filter((el) => el.offsetParent !== null);
    const i = items.indexOf(document.activeElement as HTMLElement);
    const inField = document.activeElement instanceof HTMLInputElement;
    if (e.key === "Escape" || (e.key === "Tab" && !inField)) {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      items[(i + 1) % items.length]?.focus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      items[(i - 1 + items.length) % items.length]?.focus();
    } else if (!inField && e.key === "Home") {
      e.preventDefault();
      items[0]?.focus();
    } else if (!inField && e.key === "End") {
      e.preventDefault();
      items[items.length - 1]?.focus();
    }
  };

  return (
    <div
      ref={ref}
      className={["menu", wide && "menu--wide", className].filter(Boolean).join(" ")}
      role={role}
      aria-label={label}
      style={{ position: "fixed", left: position.x, top: position.y, zIndex: "var(--z-layer)" }}
      onKeyDown={onKeyDown}
    >
      {children}
    </div>
  );
}

interface MenuItemProps {
  label: string;
  icon: string;
  meta?: string;
  kbd?: string;
  selected?: boolean;
  disabled?: boolean;
  hidden?: boolean;
  onSelect: () => void;
}

export function MenuItem({ label, icon, meta, kbd, selected, disabled, hidden, onSelect }: MenuItemProps) {
  return (
    <button
      type="button"
      className={["menu-item", selected && "is-selected"].filter(Boolean).join(" ")}
      role={selected === undefined ? "menuitem" : "menuitemradio"}
      aria-checked={selected === undefined ? undefined : selected}
      disabled={disabled}
      hidden={hidden}
      onClick={onSelect}
    >
      <Icon name={icon} />
      <span className="menu-item__text">{label}</span>
      {meta && <span className="menu-item__meta">{meta}</span>}
      {kbd && <kbd>{kbd}</kbd>}
    </button>
  );
}

export const MenuHeading = ({ children }: { children: ReactNode }) => <div className="menu__heading">{children}</div>;
export const MenuSeparator = () => <div className="menu__sep" role="separator" />;

export type MenuEntry = { label: string; icon: string; onSelect: () => void; disabled?: boolean; meta?: string; kbd?: string } | "separator";

interface MenuProps {
  label: string;
  heading?: string;
  anchor: Anchor;
  entries: MenuEntry[];
  onClose: () => void;
}

/** Menu simples a partir de uma lista de entradas (o item fecha o menu antes de agir). */
export function Menu({ label, heading, anchor, entries, onClose }: MenuProps) {
  return (
    <MenuPanel label={label} anchor={anchor} onClose={onClose}>
      {heading && <MenuHeading>{heading}</MenuHeading>}
      {entries.map((entry, i) =>
        entry === "separator" ? (
          <MenuSeparator key={`sep-${i}`} />
        ) : (
          <MenuItem
            key={entry.label}
            label={entry.label}
            icon={entry.icon}
            meta={entry.meta}
            kbd={entry.kbd}
            disabled={entry.disabled}
            onSelect={() => {
              onClose();
              entry.onSelect();
            }}
          />
        ),
      )}
    </MenuPanel>
  );
}

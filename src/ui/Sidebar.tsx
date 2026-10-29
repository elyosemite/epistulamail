// DS: navegacao.html · .sidebar, .nav-group, .nav-heading, .nav-item (.is-selected via aria-current, .has-unread), .nav-item__text, .nav-item__count
// DS: espacamento.html · .app-shell__sidebar (recolhe abaixo de 1280px; cada item leva data-tip)
// DS: pastas.html "Arrastar para uma pasta" · .nav-item.is-drop-target, .is-drop-blocked (US5); "Sinalizadas" ·
//     .nav-item.is-overdue (US6)
import type { ReactNode } from "react";
import { Icon } from "./Icon";

export function Sidebar({ label, children }: { label: string; children: ReactNode }) {
  return (
    <nav className="sidebar app-shell__sidebar" aria-label={label}>
      {children}
    </nav>
  );
}

export function NavGroup({ heading, children }: { heading?: string; children: ReactNode }) {
  return (
    <div className="nav-group">
      {heading && <div className="nav-heading">{heading}</div>}
      {children}
    </div>
  );
}

interface NavItemProps {
  icon: string;
  label: string;
  count?: number | null;
  current: boolean;
  onSelect: () => void;
  /** Estado de soltar enquanto mensagens são arrastadas. */
  drop?: "target" | "blocked" | null;
  /** Identifica o item como destino de arraste (o arraste procura `[data-drop-folder]` sob o ponteiro). */
  dropId?: string;
  /** "Atrasadas" nas sinalizadas: ícone e contador em vermelho. */
  overdue?: boolean;
  /** O contador é uma quantidade, não de não lidas: sem o destaque de `.has-unread`. */
  plainCount?: boolean;
  /** Menu de contexto (clique direito ou Shift+F10), ex.: excluir uma busca salva. */
  onMenu?: (anchor: { x: number; y: number }) => void;
}

export function NavItem({ icon, label, count, current, onSelect, drop, dropId, overdue, plainCount, onMenu }: NavItemProps) {
  const hasCount = !!count && count > 0;
  const unread = hasCount && !plainCount;
  const cls = ["nav-item", unread && "has-unread", drop === "target" && "is-drop-target", drop === "blocked" && "is-drop-blocked", overdue && "is-overdue"].filter(Boolean).join(" ");
  return (
    <button
      type="button"
      className={cls}
      aria-current={current ? "page" : undefined}
      aria-label={unread ? `${label}, ${count} não lidas` : hasCount ? `${label}, ${count}` : label}
      data-tip={label}
      onClick={onSelect}
      aria-haspopup={onMenu ? "menu" : undefined}
      onContextMenu={onMenu && ((e) => {
        e.preventDefault();
        onMenu({ x: e.clientX, y: e.clientY });
      })}
      onKeyDown={onMenu && ((e) => {
        if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
          e.preventDefault();
          const r = e.currentTarget.getBoundingClientRect();
          onMenu({ x: r.left, y: r.bottom + 4 });
        }
      })}
      data-drop-folder={dropId}
    >
      <Icon name={icon} />
      <span className="nav-item__text">{label}</span>
      {hasCount && <span className="nav-item__count" aria-hidden="true">{count!.toLocaleString("pt-BR")}</span>}
    </button>
  );
}

export function NavSkeleton({ rows = 6 }: { rows?: number }) {
  return (
    <div className="nav-group" aria-hidden="true">
      {Array.from({ length: rows }, (_, i) => (
        <span key={i} className="nav-item" style={{ cursor: "default" }}>
          <span className="skeleton skeleton--circle" />
          <span className="skeleton nav-item__text" style={{ width: `${50 + ((i * 17) % 40)}%` }} />
        </span>
      ))}
    </div>
  );
}

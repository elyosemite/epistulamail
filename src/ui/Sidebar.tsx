// DS: sidebar.html · .sidebar, .nav-group, .nav-heading, .nav-item (.is-selected via aria-current, .has-unread), .nav-item__text, .nav-item__count
// DS: espacamento.html · .app-shell__sidebar (recolhe abaixo de 1280px; cada item leva data-tip)
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
}

export function NavItem({ icon, label, count, current, onSelect }: NavItemProps) {
  const unread = !!count && count > 0;
  const cls = ["nav-item", unread && "has-unread"].filter(Boolean).join(" ");
  return (
    <button
      type="button"
      className={cls}
      aria-current={current ? "page" : undefined}
      aria-label={unread ? `${label}, ${count} não lidas` : label}
      data-tip={label}
      onClick={onSelect}
    >
      <Icon name={icon} />
      <span className="nav-item__text">{label}</span>
      {unread && <span className="nav-item__count" aria-hidden="true">{count!.toLocaleString("pt-BR")}</span>}
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

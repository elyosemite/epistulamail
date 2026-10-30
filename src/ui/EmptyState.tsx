// DS: vazios.html · .empty, .empty__art, .empty__title, .empty__desc, .empty__actions
import type { CSSProperties, ReactNode } from "react";

/** Ilustrações publicadas em vazios.html; não crie novas aqui, peça ao DS. */
const ART = {
  done: (
    <svg className="empty__art" viewBox="0 0 96 96" fill="none" aria-hidden="true">
      <rect x="12" y="28" width="72" height="48" rx="6" stroke="currentColor" strokeWidth="2.5" />
      <path d="M12 56h20l6 8h20l6-8h20" stroke="currentColor" strokeWidth="2.5" strokeLinejoin="round" />
      <path d="m38 40 7 7 13-14" stroke="var(--color-icon-success)" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  ),
  error: (
    <svg className="empty__art" viewBox="0 0 96 96" fill="none" aria-hidden="true">
      <path d="M20 30h56v40a6 6 0 0 1-6 6H26a6 6 0 0 1-6-6z" stroke="currentColor" strokeWidth="2.5" />
      <path d="M20 30 48 50l28-20" stroke="currentColor" strokeWidth="2.5" strokeLinejoin="round" />
      <circle cx="72" cy="26" r="12" fill="var(--color-surface)" stroke="var(--color-icon-danger)" strokeWidth="2.5" />
      <path d="M72 20v7M72 31v.5" stroke="var(--color-icon-danger)" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  ),
} as const;

export type EmptyArt = keyof typeof ART;

interface EmptyStateProps {
  art?: EmptyArt;
  media?: ReactNode;
  title: string;
  description?: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
  style?: CSSProperties;
}

export function EmptyState({ art, media, title, description, actions, children, style }: EmptyStateProps) {
  return (
    <div className="empty" style={style}>
      {art ? ART[art] : media}
      <h2 className="empty__title">{title}</h2>
      {description && <p className="empty__desc">{description}</p>}
      {children}
      {actions && <div className="empty__actions">{actions}</div>}
    </div>
  );
}

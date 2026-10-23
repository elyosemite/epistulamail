// DS: threads.html · .thread-list, .thread (__star, __from, __from-name, __meta, __date, __subject, __subject-text), .is-unread, .is-open, .star-btn
// DS: carregamento.html · linhas .thread com .skeleton
// DS: threads.html "Só cabeçalho" · .thread-list--headers (sem checkbox, prévia e ações rápidas)
import { forwardRef, type KeyboardEvent, type ReactNode } from "react";
import { Icon } from "./Icon";

export function ThreadList({ children, label, busy }: { children: ReactNode; label: string; busy?: boolean }) {
  return (
    <div className="thread-list thread-list--headers" role="list" aria-label={label} aria-busy={busy || undefined}>
      {children}
    </div>
  );
}

interface ThreadRowProps {
  from: string;
  subject: string;
  date: string;
  unread: boolean;
  starred: boolean;
  open: boolean;
  onOpen: () => void;
  onKeyDown?: (event: KeyboardEvent<HTMLDivElement>) => void;
}

export const ThreadRow = forwardRef<HTMLDivElement, ThreadRowProps>(function ThreadRow(
  { from, subject, date, unread, starred, open, onOpen, onKeyDown },
  ref,
) {
  const cls = ["thread", unread && "is-unread", open && "is-open"].filter(Boolean).join(" ");
  return (
    <div
      ref={ref}
      className={cls}
      role="listitem"
      tabIndex={0}
      aria-current={open || undefined}
      aria-label={`${unread ? "Não lida, " : ""}${starred ? "com estrela, " : ""}${from}, ${subject}, ${date}`}
      onClick={onOpen}
      onKeyDown={onKeyDown}
    >
      <span className="star-btn thread__star" aria-hidden="true" aria-pressed={starred}>
        <Icon name="star" />
      </span>
      <div className="thread__from"><span className="thread__from-name">{from}</span></div>
      <div className="thread__meta"><span className="thread__date">{date}</span></div>
      <div className="thread__subject"><span className="thread__subject-text">{subject}</span></div>
    </div>
  );
});

export function ThreadSkeleton({ from = "40%", subject = "70%" }: { from?: string; subject?: string }) {
  return (
    <div className="thread" style={{ cursor: "default" }} aria-hidden="true">
      <span className="skeleton skeleton--circle thread__star" />
      <span className="skeleton thread__from" style={{ width: from }} />
      <span className="skeleton thread__meta" style={{ width: "2.5rem" }} />
      <span className="skeleton thread__subject" style={{ width: subject }} />
    </div>
  );
}

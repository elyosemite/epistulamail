// DS: threads.html · .thread-list, .thread (__star, __from, __from-name, __meta, __date, __actions, __subject, __subject-text), .is-unread, .is-open, .star-btn
// DS: threads.html "Só cabeçalho" · .thread-list--headers (sem checkbox e prévia; ações rápidas opcionais)
// DS: carregamento.html · linhas .thread com .skeleton
import { forwardRef, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { IconButton } from "./Button";
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
  /** Sem estes callbacks a linha é somente leitura (estrela apagada, sem ações no hover). */
  onToggleStar?: () => void;
  onToggleRead?: () => void;
}

const stop = (fn: () => void) => (event: MouseEvent) => {
  event.stopPropagation();
  fn();
};

export const ThreadRow = forwardRef<HTMLDivElement, ThreadRowProps>(function ThreadRow(
  { from, subject, date, unread, starred, open, onOpen, onKeyDown, onToggleStar, onToggleRead },
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
      {onToggleStar ? (
        <button
          type="button"
          className="star-btn thread__star"
          aria-pressed={starred}
          aria-label={starred ? "Remover estrela" : "Marcar com estrela"}
          data-tip={starred ? "Remover estrela (S)" : "Marcar com estrela (S)"}
          tabIndex={-1}
          onClick={stop(onToggleStar)}
        >
          <Icon name="star" />
        </button>
      ) : (
        <span className="star-btn thread__star" aria-hidden="true" aria-pressed={starred}>
          <Icon name="star" />
        </span>
      )}
      <div className="thread__from"><span className="thread__from-name">{from}</span></div>
      <div className="thread__meta">
        <span className="thread__date">{date}</span>
        {onToggleRead && (
          <span className="thread__actions">
            <IconButton
              icon={unread ? "mail-open" : "mail"}
              label={unread ? "Marcar como lida" : "Marcar como não lida"}
              compact
              tabIndex={-1}
              onClick={stop(onToggleRead)}
            />
          </span>
        )}
      </div>
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

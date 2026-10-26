// DS: threads.html · .thread-list, .thread (__check, __star, __from, __from-name, __meta, __date, __actions, __subject,
//     __subject-text), .is-unread, .is-checked (mão aberta para arrastar), .is-open, .star-btn; hover com
//     --color-bg-neutral-subtle-hovered (US5); sidebar.html · .drag-ghost--follow (US6)
// DS: carregamento.html · linhas .thread com .skeleton; etiquetas.html · .label (pasta na busca); menus.html · .lozenge de prazo
// DS: threads.html "Densidade" (US7) · .thread-list (Confortável), --standard (Padrão), --compact; .thread__snippet, .thread__icons
// DS: threads.html "Lista por conversa" (US8, L48) · .thread__count ao lado dos participantes
import type { Density } from "../lib/tauri";
import { forwardRef, type CSSProperties, type KeyboardEvent, type MouseEvent, type PointerEvent, type ReactNode } from "react";
import { IconButton } from "./Button";
import { Icon } from "./Icon";

const DENSITY_CLASS: Record<Density, string> = { comfortable: "thread-list", standard: "thread-list thread-list--standard", compact: "thread-list thread-list--compact" };

export function ThreadList({ children, label, busy, density = "standard" }: { children: ReactNode; label: string; busy?: boolean; density?: Density }) {
  return (
    <div className={DENSITY_CLASS[density]} role="list" aria-label={label} aria-busy={busy || undefined}>
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
  /** Marcada para ação em lote (checkbox). */
  checked: boolean;
  onOpen: () => void;
  /** `range`: Shift pressionado (marca o intervalo). */
  onCheck: (range: boolean) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLDivElement>) => void;
  /** Começo de um possível arraste para uma pasta (US6, CA5: eventos de ponteiro). */
  onPointerDown?: (event: PointerEvent<HTMLDivElement>) => void;
  /** Ponteiro entrou ou saiu da linha (pré-busca, US9). */
  onHover?: (inside: boolean) => void;
  /** Sem estes callbacks a linha é somente leitura (estrela apagada, sem ações no hover). */
  onToggleStar?: () => void;
  onToggleRead?: () => void;
  /** Prazo da sinalização (`.lozenge`), antes da data. */
  badge?: ReactNode;
  /** Pasta da mensagem nos resultados de busca (`.label`). */
  folder?: string;
  /** Começo do texto; só aparece na densidade Confortável. */
  snippet?: string;
  hasFiles?: boolean;
  hasImages?: boolean;
  /** Mensagens da conversa (só com 2 ou mais). */
  count?: number;
}

const stop = (fn: () => void) => (event: MouseEvent) => {
  event.stopPropagation();
  fn();
};

export const ThreadRow = forwardRef<HTMLDivElement, ThreadRowProps>(function ThreadRow(
  { from, subject, date, unread, starred, open, checked, onOpen, onCheck, onKeyDown, onPointerDown, onHover, onToggleStar, onToggleRead, badge, folder, snippet, hasFiles, hasImages, count },
  ref,
) {
  const cls = ["thread", unread && "is-unread", checked && "is-checked", open && "is-open"].filter(Boolean).join(" ");
  return (
    <div
      ref={ref}
      className={cls}
      role="listitem"
      tabIndex={0}
      aria-current={open || undefined}
      aria-label={`${checked ? "Marcada, " : ""}${unread ? "Não lida, " : ""}${starred ? "com estrela, " : ""}${from}, ${count ? `${count} mensagens, ` : ""}${subject}${hasFiles ? ", com anexos" : ""}${hasImages ? ", com imagens" : ""}, ${date}`}
      onClick={onOpen}
      onKeyDown={onKeyDown}
      onPointerDown={onPointerDown}
      onPointerEnter={onHover && (() => onHover(true))}
      onPointerLeave={onHover && (() => onHover(false))}
    >
      <input
        type="checkbox"
        className="checkbox thread__check"
        aria-label={`Selecionar conversa de ${from}`}
        checked={checked}
        tabIndex={-1}
        onChange={() => undefined}
        onClick={(e) => {
          e.stopPropagation();
          onCheck(e.shiftKey);
        }}
      />
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
      <div className="thread__from">
        <span className="thread__from-name">{from}</span>
        {count !== undefined && count > 1 && <span className="thread__count">{count}</span>}
      </div>
      <div className="thread__meta">
        {badge}
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
      <div className="thread__subject">
        {folder && <span className="label">{folder}</span>}
        <span className="thread__subject-text">{subject}</span>
        {(hasFiles || hasImages) && (
          <span className="thread__icons" aria-hidden="true">
            {hasFiles && <Icon name="clip" size="sm" />}
            {hasImages && <Icon name="image" size="sm" />}
          </span>
        )}
      </div>
      {snippet && <div className="thread__snippet">{snippet}</div>}
    </div>
  );
});

export function ThreadSkeleton({ from = "40%", subject = "70%" }: { from?: string; subject?: string }) {
  return (
    <div className="thread" style={{ cursor: "default" }} aria-hidden="true">
      <span className="skeleton thread__check" style={{ width: "1rem", height: "1rem" }} />
      <span className="skeleton skeleton--circle thread__star" />
      <span className="skeleton thread__from" style={{ width: from }} />
      <span className="skeleton thread__meta" style={{ width: "2.5rem" }} />
      <span className="skeleton thread__subject" style={{ width: subject }} />
    </div>
  );
}

/** `.drag-ghost.drag-ghost--follow`: segue o ponteiro, com o cursor no canto superior esquerdo ("Mover 3 conversas"). */
export function DragGhost({ label, x, y }: { label: string; x: number; y: number }) {
  return (
    <span className="drag-ghost drag-ghost--follow" aria-hidden="true" style={{ "--x": `${x}px`, "--y": `${y}px` } as CSSProperties}>
      <Icon name="mail" />
      {label}
    </span>
  );
}

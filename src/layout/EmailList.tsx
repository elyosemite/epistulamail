// DS: threads.html · .thread-list (linha padrão com checkbox); teclado: J/K ou setas, Enter, X, S, Shift+U, Ctrl+A, Esc
// DS: sidebar.html · arrastar para uma pasta com html.is-grabbing e .drag-ghost--follow (US6, CA5)
import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { conversations } from "../lib/selection";
import type { MessageHeader } from "../lib/tauri";
import { DragGhost, ThreadList } from "../ui/Thread";
import { EmailItem } from "./EmailItem";

/** Distância mínima (px) para um clique virar arraste. */
const DRAG_THRESHOLD = 5;

export interface DragHandlers {
  /** Começou a arrastar estas mensagens (`null`: terminou ou foi cancelado). */
  onDragMessages: (ids: string[] | null) => void;
  /** Pasta (`[data-drop-folder]`) sob o ponteiro, ou `null`. */
  onDragOver: (folderId: string | null) => void;
  /** Soltou sobre uma pasta. */
  onDropMessages: (folderId: string, ids: string[]) => void;
}

interface EmailListProps extends DragHandlers {
  label: string;
  items: MessageHeader[];
  openId: string | null;
  checked: ReadonlySet<string>;
  /** Mostra a pasta de cada e-mail (busca e sinalizadas). */
  showFolder: boolean;
  /** Arrastar desligado (conta sem acesso ao servidor). */
  canDrag: boolean;
  renderBadge: (message: MessageHeader) => ReactNode;
  onOpen: (id: string) => void;
  onCheck: (id: string, range: boolean) => void;
  onCheckAll: () => void;
  onClearChecked: () => void;
  onToggleStar: (message: MessageHeader) => void;
  onToggleRead: (message: MessageHeader) => void;
}

const folderAt = (x: number, y: number) => document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-drop-folder]")?.dataset.dropFolder ?? null;

/** A lista de e-mails da pasta (ou da busca, ou de um prazo das sinalizadas). */
export function EmailList(props: EmailListProps) {
  const { label, items, openId, checked, showFolder, canDrag, renderBadge, onOpen, onCheck, onCheckAll, onClearChecked, onToggleStar, onToggleRead } = props;
  const rows = useRef(new Map<string, HTMLDivElement>());
  const [press, setPress] = useState<{ id: string; x: number; y: number } | null>(null);
  const [drag, setDrag] = useState<{ ids: string[]; x: number; y: number } | null>(null);
  // Depois de arrastar, o clique que o navegador dispara ao soltar não abre a mensagem.
  const suppressClick = useRef(false);
  const latest = useRef(props);
  latest.current = props;

  // Arraste por eventos de ponteiro: o arrastar nativo do navegador não deixa trocar o cursor.
  useEffect(() => {
    if (!press) return;
    let ids: string[] | null = null;
    let over: string | null = null;
    const stop = () => {
      document.documentElement.classList.remove("is-grabbing");
      setPress(null);
      setDrag(null);
      if (ids) latest.current.onDragMessages(null);
    };
    const onMove = (e: globalThis.PointerEvent) => {
      if (!ids) {
        if (Math.hypot(e.clientX - press.x, e.clientY - press.y) < DRAG_THRESHOLD) return;
        const { checked: marked, items: all } = latest.current;
        ids = marked.has(press.id) ? all.filter((m) => marked.has(m.id)).map((m) => m.id) : [press.id];
        document.documentElement.classList.add("is-grabbing");
        latest.current.onDragMessages(ids);
      }
      setDrag({ ids, x: e.clientX, y: e.clientY });
      const target = folderAt(e.clientX, e.clientY);
      if (target !== over) {
        over = target;
        latest.current.onDragOver(target);
      }
    };
    const onUp = (e: globalThis.PointerEvent) => {
      if (ids) {
        suppressClick.current = true;
        const target = folderAt(e.clientX, e.clientY);
        if (target) latest.current.onDropMessages(target, ids);
      }
      stop();
    };
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Escape" && ids) {
        e.preventDefault();
        stop();
      }
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", stop);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", stop);
      window.removeEventListener("keydown", onKey, true);
      document.documentElement.classList.remove("is-grabbing");
    };
  }, [press]);

  const pointerDown = (event: PointerEvent<HTMLDivElement>, message: MessageHeader) => {
    if (!canDrag || event.button !== 0 || (event.target as HTMLElement).closest("input, button")) return;
    suppressClick.current = false;
    setPress({ id: message.id, x: event.clientX, y: event.clientY });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>, index: number) => {
    const move = (to: number) => {
      const target = items[Math.max(0, Math.min(items.length - 1, to))];
      onOpen(target.id);
      rows.current.get(target.id)?.focus();
    };
    const message = items[index];
    const key = (fn: () => void) => {
      event.preventDefault();
      fn();
    };
    if (event.key === "Enter" || event.key === " ") key(() => onOpen(message.id));
    else if (event.key === "j" || event.key === "ArrowDown") key(() => move(index + 1));
    else if (event.key === "k" || event.key === "ArrowUp") key(() => move(index - 1));
    else if (event.key === "x" || event.key === "X") key(() => onCheck(message.id, event.shiftKey));
    else if (event.key === "a" && (event.ctrlKey || event.metaKey)) key(onCheckAll);
    else if (event.key === "Escape" && checked.size > 0) key(onClearChecked);
    else if (event.key === "s" && !event.ctrlKey) key(() => onToggleStar(message));
    else if (event.key === "U" && event.shiftKey) key(() => onToggleRead(message));
  };

  return (
    <>
      <ThreadList label={label}>
        {items.map((m, index) => (
          <EmailItem
            key={m.id}
            ref={(el) => { if (el) rows.current.set(m.id, el); else rows.current.delete(m.id); }}
            message={m}
            open={m.id === openId}
            checked={checked.has(m.id)}
            folderName={showFolder ? m.folderName ?? undefined : undefined}
            badge={renderBadge(m)}
            onOpen={() => {
              if (suppressClick.current) {
                suppressClick.current = false;
                return;
              }
              onOpen(m.id);
            }}
            onCheck={(range) => onCheck(m.id, range)}
            onKeyDown={(e) => onKeyDown(e, index)}
            onPointerDown={(e) => pointerDown(e, m)}
            onToggleStar={() => onToggleStar(m)}
            onToggleRead={() => onToggleRead(m)}
          />
        ))}
      </ThreadList>
      {drag && <DragGhost label={`Mover ${conversations(drag.ids.length)}`} x={drag.x} y={drag.y} />}
    </>
  );
}

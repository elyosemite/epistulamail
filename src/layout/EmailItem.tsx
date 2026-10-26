// DS: threads.html · .thread padrão com .thread__check (seleção), hover, .is-checked, .is-open; arrastável para
//     uma pasta (US5; por ponteiro, com mão aberta nas marcadas, US6); uma linha por conversa, com os participantes e
//     a quantidade (US8, D1)
import { forwardRef, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { participantsLabel } from "../lib/conversation";
import { formatListDate, senderLabel } from "../lib/format";
import type { MessageHeader } from "../lib/tauri";
import { ThreadRow } from "../ui/Thread";

interface EmailItemProps {
  message: MessageHeader;
  open: boolean;
  checked: boolean;
  /** Pasta da mensagem (resultados de busca). */
  folderName?: string;
  badge?: ReactNode;
  onOpen: () => void;
  onCheck: (range: boolean) => void;
  onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => void;
  onPointerDown: (event: PointerEvent<HTMLDivElement>) => void;
  onToggleStar: () => void;
  onToggleRead: () => void;
}

/** Um e-mail na lista. */
export const EmailItem = forwardRef<HTMLDivElement, EmailItemProps>(function EmailItem(
  { message, open, checked, folderName, badge, onOpen, onCheck, onKeyDown, onPointerDown, onToggleStar, onToggleRead },
  ref,
) {
  return (
    <ThreadRow
      ref={ref}
      from={message.thread ? participantsLabel(message.thread, senderLabel(message.fromName, message.fromAddress)) : senderLabel(message.fromName, message.fromAddress)}
      count={message.thread?.count}
      subject={message.subject || "(sem assunto)"}
      date={formatListDate(message.date)}
      unread={!message.flags.seen}
      starred={message.flags.flagged}
      open={open}
      checked={checked}
      onOpen={onOpen}
      onCheck={onCheck}
      onKeyDown={onKeyDown}
      onPointerDown={onPointerDown}
      onToggleStar={onToggleStar}
      onToggleRead={onToggleRead}
      badge={badge}
      folder={folderName}
      snippet={message.snippet}
      hasFiles={message.hasFiles}
      hasImages={message.hasImages}
    />
  );
});

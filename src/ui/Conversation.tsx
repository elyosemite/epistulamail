// DS: leitura.html "Conversa em cards" (US8, L46 e L47) · .conversation, .message.message--card, .message--collapsed
//     (.is-unread, .snip), .conversation__more com .badge, .message__quote-toggle e .message__quote
import type { KeyboardEvent, ReactNode } from "react";
import { Button } from "./Button";

export function Conversation({ children }: { children: ReactNode }) {
  return <div className="conversation">{children}</div>;
}

interface CollapsedMessageProps {
  avatar: ReactNode;
  name: string;
  snippet: string;
  time: string;
  unread: boolean;
  onExpand: () => void;
}

/** Mensagem recolhida numa linha: avatar, remetente, começo do texto e data. Clique ou Enter abre. */
export function CollapsedMessage({ avatar, name, snippet, time, unread, onExpand }: CollapsedMessageProps) {
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    e.preventDefault();
    onExpand();
  };
  return (
    <div
      className={["message--collapsed", unread && "is-unread"].filter(Boolean).join(" ")}
      role="button"
      tabIndex={0}
      aria-expanded={false}
      aria-label={`${unread ? "Não lida, " : ""}mensagem de ${name}, ${time}`}
      onClick={onExpand}
      onKeyDown={onKeyDown}
    >
      {avatar}
      <span className="snip"><b>{name}</b>{snippet && ` · ${snippet}`}</span>
      <span className="message__time">{time}</span>
    </div>
  );
}

/** "N mensagens anteriores": as do meio de uma sequência longa de recolhidas. */
export function MoreMessages({ count, onClick }: { count: number; onClick: () => void }) {
  return (
    <button type="button" className="conversation__more" aria-expanded={false} onClick={onClick}>
      <span className="badge">{count}</span>
      {count === 1 ? "mensagem anterior" : "mensagens anteriores"}
    </button>
  );
}

/** Card de uma mensagem aberta na conversa. */
export function MessageCard({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="message message--card" aria-label={label}>
      {children}
    </div>
  );
}

/** O "…" que mostra e esconde o texto citado da mensagem. */
export function QuoteToggle({ expanded, onToggle }: { expanded: boolean; onToggle: () => void }) {
  return (
    <Button
      appearance="subtle"
      compact
      className="message__quote-toggle"
      aria-expanded={expanded}
      aria-label={expanded ? "Esconder o texto citado" : "Mostrar o texto citado"}
      data-tip={expanded ? "Esconder o texto citado" : "Mostrar o texto citado"}
      onClick={onToggle}
    >
      …
    </Button>
  );
}

// DS: leitura.html · .reader (com .reader--fluid na casca em blocos, US5), .reader__head, .reader__subject, .message, .message__head, .message__from, .message__addr,
//     .message__to[aria-expanded] + .message__recipients ("Destinatários expandidos"), .message__time, .message__body
// DS: carregamento.html "Painel de leitura" · .skeleton--title, .skeleton--avatar, .skeleton--line
// DS: espacamento.html · .app-shell__back (visível só abaixo de 960px)
// Lacuna L14: leitura.html agrupa as ações do cabeçalho com .row, que só existe em docs.css; aqui usa .toolbar__end.
import { useId, useState, type ReactNode } from "react";
import { Avatar } from "./Avatar";
import { IconButton } from "./Button";
import { Icon } from "./Icon";

interface ReaderProps {
  subject: string;
  actions?: ReactNode;
  /** Etiquetas e prazo abaixo do assunto (`.reader__labels`). */
  labels?: ReactNode;
  onBack?: () => void;
  children?: ReactNode;
}

export function Reader({ subject, actions, labels, onBack, children }: ReaderProps) {
  return (
    <article className="reader reader--fluid" aria-label={subject}>
      <div className="reader__head">
        {onBack && <IconButton icon="chevron-left" label="Voltar para a lista" className="app-shell__back" onClick={onBack} />}
        <h1 className="reader__subject">{subject}</h1>
        {actions && <div className="toolbar__end">{actions}</div>}
      </div>
      {labels && <div className="reader__labels">{labels}</div>}
      {children}
    </article>
  );
}

export interface RecipientRow {
  label: string;
  value: string;
}

interface MessageHeadProps {
  fromName: string;
  fromAddress: string;
  time: string;
  /** Resumo do botão `.message__to` (ex.: "para mim, Rafael"). */
  summary?: string;
  /** Linhas do `.message__recipients` exibidas ao expandir (De, Para, Cc, Data). */
  recipients?: RecipientRow[];
  /** Outro avatar no lugar das iniciais (ex.: a foto da conta nas mensagens suas, US8). */
  avatar?: ReactNode;
  /** Selos de criptografia e assinatura abaixo do remetente (criptografia.html, L67). */
  status?: ReactNode;
}

export function MessageHead({ fromName, fromAddress, time, summary, recipients, avatar, status }: MessageHeadProps) {
  const [expanded, setExpanded] = useState(false);
  const listId = useId();
  return (
    <div className="message__head">
      {avatar ?? <Avatar name={fromName || fromAddress} seed={fromAddress} size="lg" />}
      <div style={{ minWidth: 0 }}>
        <div className="message__from">
          {fromName || fromAddress}
          {fromName && fromAddress && <span className="message__addr"> &lt;{fromAddress}&gt;</span>}
        </div>
        {summary && recipients && (
          <>
            <button type="button" className="message__to" aria-expanded={expanded} aria-controls={listId} onClick={() => setExpanded((v) => !v)}>
              {summary}
              <Icon name="chevron-down" size="sm" />
            </button>
            {expanded && (
              <dl className="message__recipients" id={listId}>
                {recipients.map((r) => (
                  <div key={r.label} style={{ display: "contents" }}>
                    <dt>{r.label}</dt>
                    <dd>{r.value}</dd>
                  </div>
                ))}
              </dl>
            )}
          </>
        )}
        {status}
      </div>
      <div className="message__time">{time}</div>
    </div>
  );
}

export function ReaderSkeleton() {
  return (
    <div className="reader reader--fluid" aria-busy="true" aria-label="Carregando mensagem">
      <span className="skeleton skeleton--title" style={{ width: "60%" }} />
      <div className="message__head">
        <span className="skeleton skeleton--avatar" />
        <div style={{ display: "grid", gap: "var(--space-075)", minWidth: 0 }}>
          <span className="skeleton" style={{ width: "12rem" }} />
          <span className="skeleton" style={{ width: "8rem" }} />
        </div>
        <span className="skeleton" style={{ width: "4rem" }} />
      </div>
      <div style={{ display: "grid", gap: "var(--space-150)", maxWidth: "var(--layout-reading-measure)" }}>
        {["95%", "88%", "92%", "55%"].map((w) => <span key={w} className="skeleton skeleton--line" style={{ width: w }} />)}
      </div>
    </div>
  );
}

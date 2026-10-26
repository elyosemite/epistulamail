// DS: composicao.html "Resposta e encaminhamento" · .compose.compose--inline (__bar, __title, __fields, __body, __foot,
//     __status), .compose__toggle, .message__quote; destinatarios.html · .chip (.is-invalid, __name, __remove),
//     .input--line.chip-input com .input__prefix; botoes.html · .btn-split
// DS: composicao.html "Envio" (US7) · .compose__quote-toggle, .compose__attachments com .attachment, .compose__hint,
//     .compose.is-sending; carregamento.html · .spinner--sm no botão
// DS: criptografia.html "Escrever com criptografia" (US13) · .chip__key (--missing) e a faixa .compose__security (L68)
import { useId, useState, type KeyboardEvent, type ReactNode } from "react";
import type { SendCheck } from "../lib/compose";
import { attachmentKind, avatarColor, formatBytes, initials } from "../lib/format";
import { addRecipients, parseRecipient } from "../lib/recipients";
import type { AttachmentInfo } from "../lib/tauri";
import { IconButton } from "./Button";
import { Icon } from "./Icon";

interface RecipientFieldProps {
  label: string;
  values: string[];
  onChange: (values: string[]) => void;
  trailing?: ReactNode;
  /** Com o cadeado ligado: quem tem chave aceita (endereço em minúsculas). */
  keys?: ReadonlyMap<string, boolean> | null;
}

/** Destinatários em chips: `,`, `;`, Enter ou sair do campo confirmam; Backspace no campo vazio tira o último. */
export function RecipientField({ label, values, onChange, trailing, keys }: RecipientFieldProps) {
  const [text, setText] = useState("");
  const id = useId();
  const commit = () => {
    if (!text.trim()) return;
    onChange(addRecipients(values, text));
    setText("");
  };
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === "," || e.key === ";") {
      e.preventDefault();
      commit();
    } else if (e.key === "Backspace" && !text && values.length > 0) {
      onChange(values.slice(0, -1));
    }
  };
  return (
    <div className="input input--line chip-input">
      <label className="input__prefix" htmlFor={id}>{label}</label>
      {values.map((raw, i) => {
        const r = parseRecipient(raw);
        const display = r.name || r.address;
        return (
          <span key={`${raw}-${i}`} className={["chip", !r.valid && "is-invalid"].filter(Boolean).join(" ")} title={r.valid ? r.address : "Endereço inválido"}>
            {r.valid ? (
              <span className={["avatar", avatarColor(r.address) && `avatar--${avatarColor(r.address)}`].filter(Boolean).join(" ")} aria-hidden="true">{initials(display)}</span>
            ) : (
              <Icon name="warning" size="sm" />
            )}
            <span className="chip__name">{display}</span>
            {keys && r.valid && (keys.get(r.address.toLowerCase()) ? (
              <span className="chip__key" role="img" aria-label="Tem chave aceita" data-tip="Tem chave aceita"><Icon name="lock" /></span>
            ) : (
              <span className="chip__key chip__key--missing" role="img" aria-label="Sem chave" data-tip="Sem chave"><Icon name="lock-open" /></span>
            ))}
            <button className="chip__remove" type="button" aria-label={`Remover ${display}`} onClick={() => onChange(values.filter((_, j) => j !== i))}>
              <Icon name="x" size="sm" />
            </button>
          </span>
        );
      })}
      <input id={id} value={text} aria-label={`Adicionar em ${label}`} onChange={(e) => setText(e.target.value)} onKeyDown={onKeyDown} onBlur={commit} />
      {trailing}
    </div>
  );
}

export interface ComposeValue {
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  body: string;
}

export interface QuoteView {
  from: string;
  sent: string;
  to: string;
  cc: string;
  subject: string;
  text: string;
}

/** Citação da resposta (D5): cabeçalho "Em …, … escreveu:" e as linhas com `>`. */
export interface ReplyQuote {
  header: string;
  lines: string;
}

interface InlineComposeProps {
  title: string;
  value: ComposeValue;
  onChange: (value: ComposeValue) => void;
  /** Encaminhamento: a mensagem original, sempre visível. */
  quote?: QuoteView | null;
  /** Resposta: a citação, recolhida no botão "…" (`null` depois de removida). */
  replyQuote?: ReplyQuote | null;
  onRemoveReplyQuote?: () => void;
  /** Encaminhamento: anexos da original que vão junto, cada um com "Remover". */
  attachments?: AttachmentInfo[];
  onRemoveAttachment?: (index: number) => void;
  status: string;
  /** Rótulo do botão de descartar ("Descartar" ou "Remover"). */
  discardLabel: string;
  onDiscard: () => void;
  bodyLabel: string;
  /** Enviar ativo ou o motivo do bloqueio (ao lado do botão). */
  sendCheck: SendCheck;
  sending: boolean;
  onSend: () => void;
  /** US13: chaves dos destinatários (com o cadeado ligado), a faixa de segurança e o rótulo do Enviar. */
  recipientKeys?: ReadonlyMap<string, boolean> | null;
  security?: ReactNode;
  sendLabel?: string;
  sendIcon?: string;
}

export function InlineCompose(props: InlineComposeProps) {
  const { title, value, onChange, quote, replyQuote, attachments, status, discardLabel, onDiscard, bodyLabel, sendCheck, sending, onSend, recipientKeys } = props;
  const [copies, setCopies] = useState(value.cc.length > 0 || value.bcc.length > 0);
  const [quoteOpen, setQuoteOpen] = useState(false);
  const hintId = useId();
  const set = (change: Partial<ComposeValue>) => onChange({ ...value, ...change });
  const onKeyDown = (e: KeyboardEvent<HTMLFormElement>) => {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (sendCheck.ok && !sending) onSend();
    }
  };
  return (
    <form className={["compose compose--inline", sending && "is-sending"].filter(Boolean).join(" ")} aria-label={title} aria-busy={sending || undefined} onSubmit={(e) => e.preventDefault()} onKeyDown={onKeyDown}>
      <div className="compose__bar">
        <span className="compose__title">{title}</span>
        <IconButton icon="trash" label={discardLabel} compact onClick={onDiscard} />
      </div>
      <div className="compose__fields">
        <RecipientField
          label="Para"
          values={value.to}
          onChange={(to) => set({ to })}
          keys={recipientKeys}
          trailing={
            !copies && (
              <button className="btn btn--subtle btn--compact compose__toggle" type="button" aria-expanded={copies} onClick={() => setCopies(true)}>
                Cc/Cco
              </button>
            )
          }
        />
        {copies && <RecipientField label="Cc" values={value.cc} onChange={(cc) => set({ cc })} keys={recipientKeys} />}
        {copies && <RecipientField label="Cco" values={value.bcc} onChange={(bcc) => set({ bcc })} keys={recipientKeys} />}
        <label className="input input--line">
          <span className="input__prefix">Assunto</span>
          <input value={value.subject} maxLength={1000} onChange={(e) => set({ subject: e.target.value })} />
        </label>
      </div>
      <div className="compose__body">
        <textarea aria-label={bodyLabel} value={value.body} onChange={(e) => set({ body: e.target.value })} />
      </div>
      {replyQuote && (
        <>
          <button
            className="btn btn--subtle btn--compact compose__quote-toggle"
            type="button"
            aria-expanded={quoteOpen}
            aria-label={quoteOpen ? "Esconder a mensagem citada" : "Mostrar a mensagem citada"}
            data-tip={quoteOpen ? "Esconder a mensagem citada" : "Mostrar a mensagem citada"}
            onClick={() => setQuoteOpen((o) => !o)}
          >
            …
          </button>
          {quoteOpen && (
            <div className="message__quote">
              <p style={{ marginTop: 0 }}>{replyQuote.header}</p>
              <p style={{ whiteSpace: "pre-wrap" }}>{replyQuote.lines}</p>
              {props.onRemoveReplyQuote && (
                <button className="btn btn--subtle btn--compact" type="button" onClick={props.onRemoveReplyQuote}>Não citar a mensagem</button>
              )}
            </div>
          )}
        </>
      )}
      {attachments && attachments.length > 0 && (
        <div className="compose__attachments" role="list" aria-label="Anexos">
          {attachments.map((a, i) => {
            const kind = attachmentKind(a.mimeType);
            return (
              <div key={`${a.name}-${i}`} className="attachment attachment--static" role="listitem">
                <span className={["attachment__icon", kind !== "other" && `attachment__icon--${kind}`].filter(Boolean).join(" ")}>
                  <Icon name={kind === "img" ? "image" : "file"} size="lg" />
                </span>
                <span style={{ minWidth: 0, display: "grid" }}>
                  <span className="attachment__name">{a.name}</span>
                  <span className="attachment__meta">{formatBytes(a.size)}</span>
                </span>
                {props.onRemoveAttachment && (
                  <span className="attachment__action">
                    <IconButton icon="x" label={`Remover ${a.name}`} data-tip="Remover" compact onClick={() => props.onRemoveAttachment!(i)} />
                  </span>
                )}
              </div>
            );
          })}
        </div>
      )}
      {quote && (
        <div className="message__quote">
          <dl>
            <dt>De</dt><dd>{quote.from}</dd>
            <dt>Enviado</dt><dd>{quote.sent}</dd>
            <dt>Para</dt><dd>{quote.to}</dd>
            {quote.cc && (<><dt>Cc</dt><dd>{quote.cc}</dd></>)}
            <dt>Assunto</dt><dd>{quote.subject}</dd>
          </dl>
          {quote.text && <p>{quote.text}</p>}
        </div>
      )}
      {props.security}
      <div className="compose__foot">
        <button
          className="btn btn--primary"
          type="button"
          disabled={!sendCheck.ok || sending}
          aria-describedby={sendCheck.ok ? undefined : hintId}
          data-tip="Enviar (Ctrl+Enter)"
          onClick={onSend}
        >
          {sending ? <span className="spinner spinner--sm" aria-hidden="true" /> : <Icon name={props.sendIcon ?? "send"} />}
          {sending ? "Enviando…" : props.sendLabel ?? "Enviar"}
        </button>
        {!sendCheck.ok && (
          <span className="compose__hint" id={hintId}><Icon name="error" size="sm" />{sendCheck.reason}</span>
        )}
        <span className="compose__status" role="status">{status}</span>
        <IconButton icon="trash" label={discardLabel} onClick={onDiscard} />
      </div>
    </form>
  );
}

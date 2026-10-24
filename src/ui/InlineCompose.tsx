// DS: composicao.html "Resposta e encaminhamento" · .compose.compose--inline (__bar, __title, __fields, __body, __foot,
//     __status), .compose__toggle, .message__quote; destinatarios.html · .chip (.is-invalid, __name, __remove),
//     .input--line.chip-input com .input__prefix; botoes.html · .btn-split
import { useId, useState, type KeyboardEvent, type ReactNode } from "react";
import { addRecipients, parseRecipient } from "../lib/recipients";
import { avatarColor, initials } from "../lib/format";
import { IconButton } from "./Button";
import { Icon } from "./Icon";

interface RecipientFieldProps {
  label: string;
  values: string[];
  onChange: (values: string[]) => void;
  trailing?: ReactNode;
}

/** Destinatários em chips: `,`, `;`, Enter ou sair do campo confirmam; Backspace no campo vazio tira o último. */
export function RecipientField({ label, values, onChange, trailing }: RecipientFieldProps) {
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

interface InlineComposeProps {
  title: string;
  value: ComposeValue;
  onChange: (value: ComposeValue) => void;
  quote?: QuoteView | null;
  status: string;
  /** Rótulo do botão de descartar ("Descartar" ou "Remover"). */
  discardLabel: string;
  onDiscard: () => void;
  bodyLabel: string;
}

export function InlineCompose({ title, value, onChange, quote, status, discardLabel, onDiscard, bodyLabel }: InlineComposeProps) {
  const [copies, setCopies] = useState(value.cc.length > 0 || value.bcc.length > 0);
  const set = (change: Partial<ComposeValue>) => onChange({ ...value, ...change });
  return (
    <form className="compose compose--inline" aria-label={title} onSubmit={(e) => e.preventDefault()}>
      <div className="compose__bar">
        <span className="compose__title">{title}</span>
        <IconButton icon="trash" label={discardLabel} compact onClick={onDiscard} />
      </div>
      <div className="compose__fields">
        <RecipientField
          label="Para"
          values={value.to}
          onChange={(to) => set({ to })}
          trailing={
            !copies && (
              <button className="btn btn--subtle btn--compact compose__toggle" type="button" aria-expanded={copies} onClick={() => setCopies(true)}>
                Cc/Cco
              </button>
            )
          }
        />
        {copies && <RecipientField label="Cc" values={value.cc} onChange={(cc) => set({ cc })} />}
        {copies && <RecipientField label="Cco" values={value.bcc} onChange={(bcc) => set({ bcc })} />}
        <label className="input input--line">
          <span className="input__prefix">Assunto</span>
          <input value={value.subject} maxLength={1000} onChange={(e) => set({ subject: e.target.value })} />
        </label>
      </div>
      <div className="compose__body">
        <textarea aria-label={bodyLabel} value={value.body} onChange={(e) => set({ body: e.target.value })} />
      </div>
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
      <div className="compose__foot">
        <div className="btn-split" data-tip="O envio chega na próxima versão">
          <button className="btn btn--primary" type="button" disabled>Enviar</button>
          <button className="btn btn--primary" type="button" disabled aria-label="Opções de envio"><Icon name="chevron-down" /></button>
        </div>
        <span className="compose__status" role="status">{status}</span>
        <IconButton icon="trash" label={discardLabel} onClick={onDiscard} />
      </div>
    </form>
  );
}

// Composição de DS: composicao.html (.compose--inline, .message__quote; "Envio": citação recolhida, anexos do
// encaminhamento, motivo do bloqueio e "Enviando…") + modais.html (confirmar descarte e "Enviar sem assunto?")
// Resposta, resposta a todos, encaminhamento e e-mail novo (US4, CA8), com envio (US7, CA1–CA3). O rascunho fica no
// banco local até o envio dar certo.
import { useEffect, useRef, useState } from "react";
import { checkSend, outgoingBody, quoteHeader, quotedLines } from "../../lib/compose";
import { formatLongDate } from "../../lib/format";
import { api, isAppError, type Account, type AttachmentInfo, type DraftKind, type Quote } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { InlineCompose, type ComposeValue } from "../../ui/InlineCompose";
import { Modal } from "../../ui/Modal";
import { Spinner } from "../../ui/Spinner";

const SAVE_DELAY_MS = 2000;

const TITLE: Record<DraftKind, string> = {
  new: "Nova mensagem",
  reply: "Responder",
  reply_all: "Responder a todos",
  forward: "Encaminhar",
};

const EMPTY: ComposeValue = { to: [], cc: [], bcc: [], subject: "", body: "" };

interface ComposeCardProps {
  account: Account;
  kind: DraftKind;
  /** Mensagem respondida ou encaminhada (`null` em e-mail novo). */
  messageId: string | null;
  /** Anexos da mensagem original (encaminhar leva todos, e cada um pode ser removido). */
  originalAttachments?: AttachmentInfo[];
  onClose: () => void;
  /** O envio foi agendado (6 s para desfazer): o card fecha e o toast assume. */
  onSent: (sendId: string) => void;
}

const time = () => new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });

export function ComposeCard({ account, kind, messageId, originalAttachments = [], onClose, onSent }: ComposeCardProps) {
  const [value, setValue] = useState<ComposeValue | null>(null);
  const [quote, setQuote] = useState<Quote | null>(null);
  const [keepQuote, setKeepQuote] = useState(true);
  const [kept, setKept] = useState<number[]>(() => (kind === "forward" ? originalAttachments.map((_, i) => i) : []));
  const [status, setStatus] = useState("Rascunho neste computador");
  const [confirm, setConfirm] = useState<"discard" | "no-subject" | null>(null);
  const [sending, setSending] = useState(false);
  const pending = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latest = useRef<ComposeValue | null>(null);
  latest.current = value;

  useEffect(() => {
    let cancelled = false;
    const template = kind === "new" || !messageId ? Promise.resolve(null) : api.composeTemplate(messageId, kind);
    Promise.all([template, api.getDraft(account.id, kind, messageId)])
      .then(([t, draft]) => {
        if (cancelled) return;
        setValue(draft ? { to: draft.to, cc: draft.cc, bcc: draft.bcc, subject: draft.subject, body: draft.body } : t ? { ...EMPTY, to: t.to, cc: t.cc, subject: t.subject } : EMPTY);
        if (t?.quote) setQuote(t.quote);
        if (draft) setStatus("Rascunho recuperado deste computador");
      })
      .catch(() => {
        if (!cancelled) setValue(EMPTY);
      });
    return () => {
      cancelled = true;
    };
  }, [account.id, kind, messageId]);

  const save = (v: ComposeValue) =>
    api
      .saveDraft(account.id, kind, messageId, v)
      .then(() => setStatus(`Rascunho salvo neste computador às ${time()}`))
      .catch(() => setStatus("Não foi possível salvar o rascunho"));

  // Salvar o que estiver pendente ao fechar o card (troca de mensagem, de conta ou de pasta).
  useEffect(
    () => () => {
      if (pending.current && latest.current) {
        clearTimeout(pending.current);
        void api.saveDraft(account.id, kind, messageId, latest.current).catch(() => undefined);
      }
    },
    [account.id, kind, messageId],
  );

  const change = (v: ComposeValue) => {
    setValue(v);
    setStatus("Editando…");
    if (pending.current) clearTimeout(pending.current);
    pending.current = setTimeout(() => {
      pending.current = null;
      void save(v);
    }, SAVE_DELAY_MS);
  };

  const discard = async () => {
    if (pending.current) clearTimeout(pending.current);
    pending.current = null;
    await api.deleteDraft(account.id, kind, messageId).catch(() => undefined);
    onClose();
  };

  const attachments = kept.map((i) => originalAttachments[i]).filter(Boolean);
  const attachmentBytes = attachments.reduce((sum, a) => sum + a.size, 0);
  const sendCheck = value ? checkSend(value, attachmentBytes) : { ok: false as const, reason: "" };

  /** O rascunho é salvo antes: se o envio for desfeito ou falhar, o texto volta igual. */
  const send = async (confirmed = false) => {
    if (!value || !sendCheck.ok || sending) return;
    if (!confirmed && !value.subject.trim()) return setConfirm("no-subject");
    setConfirm(null);
    setSending(true);
    if (pending.current) clearTimeout(pending.current);
    pending.current = null;
    await save(value);
    try {
      const sendId = await api.sendMessage({
        accountId: account.id,
        kind,
        messageId,
        to: value.to,
        cc: value.cc,
        bcc: value.bcc,
        subject: value.subject,
        body: outgoingBody(kind, value.body, quote, keepQuote),
        keepAttachments: kept,
      });
      onSent(sendId);
    } catch (error) {
      setSending(false);
      setStatus(isAppError(error) ? error.message : "Não foi possível enviar. Tente de novo.");
    }
  };

  const hasText = !!value && (value.body.trim() !== "" || (kind === "new" && (value.to.length > 0 || value.subject.trim() !== "")));

  if (!value) {
    return <Spinner label="Preparando a resposta" />;
  }
  const replying = kind === "reply" || kind === "reply_all";
  return (
    <>
      <InlineCompose
        title={TITLE[kind]}
        value={value}
        onChange={change}
        quote={kind === "forward" && quote ? { ...quote, sent: formatLongDate(quote.sent) || quote.sent } : null}
        replyQuote={replying && quote && keepQuote && quote.text.trim() ? { header: quoteHeader(quote), lines: quotedLines(quote.text) } : null}
        onRemoveReplyQuote={() => setKeepQuote(false)}
        attachments={kind === "forward" ? attachments : undefined}
        onRemoveAttachment={(i) => setKept((all) => all.filter((_, j) => j !== i))}
        status={status}
        discardLabel={kind === "forward" ? "Remover" : "Descartar"}
        bodyLabel={kind === "forward" ? "Texto do encaminhamento" : kind === "new" ? "Texto da mensagem" : "Texto da resposta"}
        onDiscard={() => (hasText ? setConfirm("discard") : void discard())}
        sendCheck={sendCheck}
        sending={sending}
        onSend={() => void send()}
      />
      {confirm === "discard" && (
        <Modal
          title="Descartar este rascunho?"
          icon="trash"
          initialFocus="[data-cancel]"
          onClose={() => setConfirm(null)}
          footer={
            <>
              <Button appearance="subtle" data-cancel onClick={() => setConfirm(null)}>Cancelar</Button>
              <Button appearance="danger" onClick={() => void discard()}>Descartar</Button>
            </>
          }
        >
          <p>O texto escrito será apagado deste computador.</p>
        </Modal>
      )}
      {confirm === "no-subject" && (
        <Modal
          title="Enviar sem assunto?"
          initialFocus="[data-cancel]"
          onClose={() => setConfirm(null)}
          footer={
            <>
              <Button appearance="subtle" data-cancel onClick={() => setConfirm(null)}>Voltar e escrever</Button>
              <Button appearance="primary" onClick={() => void send(true)}>Enviar assim</Button>
            </>
          }
        >
          <p>Quem receber vai ver "(sem assunto)" na caixa de entrada.</p>
        </Modal>
      )}
    </>
  );
}

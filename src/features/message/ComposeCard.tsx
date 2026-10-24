// Composição de DS: composicao.html (.compose--inline, .message__quote) + modais.html (confirmar descarte)
// Resposta, resposta a todos, encaminhamento e e-mail novo (US4, CA8). Só interface: o rascunho fica no banco local.
import { useEffect, useRef, useState } from "react";
import { formatLongDate } from "../../lib/format";
import { api, type Account, type DraftKind } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { InlineCompose, type ComposeValue, type QuoteView } from "../../ui/InlineCompose";
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
  onClose: () => void;
}

const time = () => new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });

export function ComposeCard({ account, kind, messageId, onClose }: ComposeCardProps) {
  const [value, setValue] = useState<ComposeValue | null>(null);
  const [quote, setQuote] = useState<QuoteView | null>(null);
  const [status, setStatus] = useState("Rascunho neste computador");
  const [confirm, setConfirm] = useState(false);
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
        if (t?.quote) setQuote({ ...t.quote, sent: formatLongDate(t.quote.sent) || t.quote.sent });
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

  const hasText = !!value && (value.body.trim() !== "" || (kind === "new" && (value.to.length > 0 || value.subject.trim() !== "")));

  if (!value) {
    return <Spinner label="Preparando a resposta" />;
  }
  return (
    <>
      <InlineCompose
        title={TITLE[kind]}
        value={value}
        onChange={change}
        quote={quote}
        status={status}
        discardLabel={kind === "forward" ? "Remover" : "Descartar"}
        bodyLabel={kind === "forward" ? "Texto do encaminhamento" : kind === "new" ? "Texto da mensagem" : "Texto da resposta"}
        onDiscard={() => (hasText ? setConfirm(true) : void discard())}
      />
      {confirm && (
        <Modal
          title="Descartar este rascunho?"
          icon="trash"
          initialFocus="[data-cancel]"
          onClose={() => setConfirm(false)}
          footer={
            <>
              <Button appearance="subtle" data-cancel onClick={() => setConfirm(false)}>Cancelar</Button>
              <Button appearance="danger" onClick={() => void discard()}>Descartar</Button>
            </>
          }
        >
          <p>O texto escrito será apagado deste computador.</p>
        </Modal>
      )}
    </>
  );
}

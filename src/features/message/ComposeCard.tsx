// Composição de DS: composicao.html (.compose--inline, .message__quote; "Envio": citação recolhida, anexos do
// encaminhamento, motivo do bloqueio e "Enviando…") + modais.html (confirmar descarte e "Enviar sem assunto?")
// + criptografia.html "Escrever com criptografia" (US13: .compose__security e o aviso de destinatário sem chave)
// Resposta, resposta a todos, encaminhamento e e-mail novo (US4, CA8), com envio (US7, CA1–CA3). O rascunho fica no
// banco local até o envio dar certo.
import { useEffect, useRef, useState } from "react";
import { checkSend, outgoingBody, quoteHeader, quotedLines } from "../../lib/compose";
import { keyName, sendLabel } from "../../lib/crypto";
import { formatLongDate } from "../../lib/format";
import { api, isAppError, type Account, type AttachmentInfo, type DraftKind, type Quote } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { ComposeSecurityBar } from "../../ui/ComposeSecurityBar";
import { InlineCompose, type ComposeValue } from "../../ui/InlineCompose";
import { Modal } from "../../ui/Modal";
import { parseRecipient } from "../../lib/recipients";
import { SectionMessage } from "../../ui/SectionMessage";
import { Spinner } from "../../ui/Spinner";
import { useComposeSecurity } from "./useComposeSecurity";

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
  /** Resposta a uma mensagem criptografada: o cadeado começa ligado (US13, CA6). */
  replyToEncrypted?: boolean;
  onClose: () => void;
  /** O envio foi agendado (6 s para desfazer): o card fecha e o toast assume. */
  onSent: (sendId: string) => void;
}

const time = () => new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });

export function ComposeCard({ account, kind, messageId, originalAttachments = [], replyToEncrypted = false, onClose, onSent }: ComposeCardProps) {
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
  const recipients = value ? [...value.to, ...value.cc, ...value.bcc] : [];
  const crypto = useComposeSecurity(account.id, recipients, replyToEncrypted && kind !== "forward");
  const { encrypt, sign, missing } = crypto.security;

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
  const baseCheck = value ? checkSend(value, attachmentBytes) : { ok: false as const, reason: "" };
  const sendCheck = baseCheck.ok && encrypt && missing.length > 0 ? { ok: false as const, reason: "Resolva o destinatário sem chave para enviar" } : baseCheck;

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
        encrypt,
        sign,
        technology: crypto.technology ?? "openpgp",
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
        recipientKeys={crypto.keys}
        sendLabel={sendLabel({ encrypt, sign })}
        sendIcon={encrypt ? "lock" : sign ? "signature" : undefined}
        security={
          crypto.enabled && (
            <>
              {encrypt && missing.length > 0 && <MissingKeys missing={missing} crypto={crypto} onRemove={() => change(removeAddresses(value, missing))} />}
              <ComposeSecurityBar
                encrypt={encrypt}
                sign={sign}
                technology={crypto.technology ?? "openpgp"}
                onSwitch={crypto.canSwitch ? () => crypto.setTechnology(crypto.technology === "smime" ? "openpgp" : "smime") : undefined}
                onEncrypt={(on) => crypto.setChoice((c) => ({ ...c, encrypt: on }))}
                onSign={(on) => crypto.setChoice((c) => ({ ...c, sign: on }))}
              />
            </>
          )
        }
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

/** Tira os endereços (em minúsculas) de Para, Cc e Cco. */
function removeAddresses(value: ComposeValue, addresses: string[]): ComposeValue {
  const keep = (list: string[]) => list.filter((r) => !addresses.includes(parseRecipient(r).address.toLowerCase()));
  return { ...value, to: keep(value.to), cc: keep(value.cc), bcc: keep(value.bcc) };
}

/** Aviso com o cadeado ligado e destinatários sem chave: buscar (o clique é o consentimento), aceitar o que achou, enviar sem criptografia ou tirar. */
function MissingKeys({ missing, crypto, onRemove }: { missing: string[]; crypto: ReturnType<typeof useComposeSecurity>; onRemove: () => void }) {
  const one = missing.length === 1;
  if (crypto.technology === "smime") {
    const actions = [
      ...(crypto.canSwitch ? [{ label: "Usar OpenPGP", onClick: () => crypto.setTechnology("openpgp") }] : []),
      { label: "Enviar sem criptografia", onClick: () => crypto.setChoice((c) => ({ ...c, encrypt: false })) },
      { label: one ? "Tirar este destinatário" : "Tirar estes destinatários", onClick: onRemove },
    ];
    return (
      <div style={{ margin: "0 var(--space-200) var(--space-150)" }}>
        <SectionMessage appearance="warning" title={one ? `${missing[0]} não tem certificado S/MIME` : `${missing.length} destinatários não têm certificado S/MIME`} actions={actions}>
          Sem um certificado confiável, a mensagem não pode ser criptografada para ninguém. O certificado chega sozinho quando a pessoa manda uma mensagem assinada com S/MIME, ou pode ser importado no gerenciador de certificados.
        </SectionMessage>
      </div>
    );
  }
  const actions: Array<{ label: string; onClick: () => void }> = [];
  if (crypto.found.length > 0) {
    actions.push({ label: one ? "Aceitar a chave encontrada" : "Aceitar as chaves encontradas", onClick: () => void crypto.accept(crypto.found.map((k) => k.fingerprint)) });
  } else if (crypto.settings?.searchOnline !== "never" && !crypto.searching && !crypto.searchedAll) {
    actions.push({ label: one ? "Buscar a chave na internet" : "Buscar as chaves na internet", onClick: () => void crypto.search(missing) });
  }
  actions.push({ label: "Enviar sem criptografia", onClick: () => crypto.setChoice((c) => ({ ...c, encrypt: false })) });
  actions.push({ label: one ? "Tirar este destinatário" : "Tirar estes destinatários", onClick: onRemove });
  return (
    <div style={{ margin: "0 var(--space-200) var(--space-150)" }}>
      <SectionMessage appearance="warning" title={one ? `${missing[0]} não tem chave` : `${missing.length} destinatários não têm chave`} actions={actions}>
        {crypto.searching
          ? "Buscando no domínio do destinatário e em keys.openpgp.org…"
          : crypto.found.length > 0
            ? `Achamos ${crypto.found.map((k) => `a chave de ${keyName(k)} (${k.fingerprint.slice(-16).match(/.{4}/g)?.join(" ")})`).join(", ")}. Aceite só se confiar que a chave é mesmo da pessoa; dá para conferir a impressão digital no gerenciador de chaves.`
            : crypto.searchedAll && crypto.settings?.searchOnline !== "never"
              ? "Nenhuma chave publicada. Peça a chave pública à pessoa ou envie sem criptografia."
              : `Sem ${one ? "a chave" : "as chaves"}, a mensagem não pode ser criptografada para ninguém.${crypto.settings?.searchOnline === "ask" ? " Buscar na internet avisa quem responde a busca com quem você vai falar." : ""}`}
      </SectionMessage>
    </div>
  );
}

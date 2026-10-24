// Composição de DS: leitura.html (.reader, cabeçalho, destinatários, .message__body, .message__frame),
// mensagens.html (.section-message "Imagens externas bloqueadas", avisos e erros), anexos.html (.attachment--static),
// carregamento.html (skeleton da leitura), vazios.html (.empty), composicao.html (.compose--inline: resposta abaixo da
// mensagem, encaminhamento acima), menus.html (.lozenge de prazo em .reader__labels)
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { formatFullDate, formatPeople, paragraphs, recipientsSummary } from "../../lib/format";
import { api, isAccessLost, isAppError, type Account, type AppError, type DraftKind, type MessageFlags, type MessageView as View } from "../../lib/tauri";
import { AttachmentList } from "../../ui/Attachment";
import { EmptyState } from "../../ui/EmptyState";
import { MessageFrame } from "../../ui/MessageFrame";
import { MessageHead, Reader, ReaderSkeleton, type RecipientRow } from "../../ui/Reader";
import { SectionMessage } from "../../ui/SectionMessage";
import { useToast } from "../../ui/Toast";
import { ComposeCard } from "./ComposeCard";

/** Card de resposta ou encaminhamento aberto no painel de leitura. */
export interface ComposeState {
  kind: DraftKind;
  messageId: string | null;
}

type State = { status: "loading" } | { status: "ready"; view: View } | { status: "error"; error: AppError };

interface MessageViewProps {
  account: Account;
  messageId: string | null;
  /** Prazo da sinalização da mensagem aberta (lozenge), vindo da lista. */
  badge?: ReactNode;
  compose: ComposeState | null;
  onCloseCompose: () => void;
  onLoaded: (id: string, flags: MessageFlags) => void;
  onAccessLost: () => void;
  onBack: () => void;
}

export function MessageView({ account, messageId, badge, compose, onCloseCompose, onLoaded, onAccessLost, onBack }: MessageViewProps) {
  const [state, setState] = useState<State>({ status: "loading" });
  const [allowRemote, setAllowRemote] = useState(false);
  const toast = useToast();

  const load = useCallback(async (id: string, remote: boolean, signal: { cancelled: boolean }) => {
    setState({ status: "loading" });
    try {
      const view = await api.getMessage(id, remote);
      if (signal.cancelled) return;
      setState({ status: "ready", view });
      onLoaded(id, view.flags);
    } catch (error) {
      if (signal.cancelled) return;
      if (isAccessLost(error)) onAccessLost();
      setState({ status: "error", error: isAppError(error) ? error : { kind: "ImapConnection", message: "Tente de novo em instantes." } });
    }
  }, [onLoaded, onAccessLost]);

  useEffect(() => {
    setAllowRemote(false);
    if (!messageId) return;
    const signal = { cancelled: false };
    void load(messageId, false, signal);
    return () => { signal.cancelled = true; };
  }, [messageId, load]);

  const openLink = useCallback((url: string) => {
    api.openExternalLink(url).catch(() =>
      toast({ kind: "warning", title: "Este link não pode ser aberto", description: "Só links da web e de e-mail saem do Epistula." }),
    );
  }, [toast]);

  if (compose?.kind === "new") {
    return (
      <div className="reader">
        <ComposeCard key="new" account={account} kind="new" messageId={null} onClose={onCloseCompose} />
      </div>
    );
  }
  if (!messageId) {
    return <EmptyState title="Nenhuma mensagem aberta" description="Escolha uma mensagem na lista para ler." />;
  }
  if (state.status === "loading") return <ReaderSkeleton />;
  if (state.status === "error") {
    if (state.error.kind === "MessageNotFound") {
      return (
        <div className="reader">
          <SectionMessage appearance="warning" title="Esta mensagem não está mais nesta pasta">
            Ela pode ter sido movida ou excluída em outro dispositivo. Atualize a lista.
          </SectionMessage>
        </div>
      );
    }
    return (
      <div className="reader">
        <SectionMessage
          appearance="error"
          title="Não foi possível abrir a mensagem"
          actions={state.error.kind === "MessageTooLarge" ? undefined : [{ label: "Tentar de novo", onClick: () => void load(messageId, allowRemote, { cancelled: false }) }]}
        >
          {state.error.message}
        </SectionMessage>
      </div>
    );
  }

  const { view } = state;
  const card = compose && compose.messageId === messageId && (
    <ComposeCard key={`${compose.kind}-${messageId}`} account={account} kind={compose.kind} messageId={messageId} onClose={onCloseCompose} />
  );
  const subject = view.subject || "(sem assunto)";
  const date = formatFullDate(view.date);
  const recipients: RecipientRow[] = [
    { label: "De", value: formatPeople([{ name: view.fromName, address: view.fromAddress }]) },
    ...(view.to.length ? [{ label: "Para", value: formatPeople(view.to) }] : []),
    ...(view.cc.length ? [{ label: "Cc", value: formatPeople(view.cc) }] : []),
    { label: "Data", value: date },
  ];

  const showImages = () => {
    setAllowRemote(true);
    void load(view.id, true, { cancelled: false });
  };

  return (
    <Reader subject={subject} labels={badge} onBack={onBack}>
      {compose?.kind === "forward" && card}
      {view.blockedRemoteImages > 0 && !allowRemote && (
        <SectionMessage title="Imagens externas bloqueadas" actions={[{ label: "Mostrar imagens", onClick: showImages }]}>
          Para proteger sua privacidade, {view.blockedRemoteImages === 1 ? "1 imagem desta mensagem não foi carregada" : `${view.blockedRemoteImages} imagens desta mensagem não foram carregadas`}.
        </SectionMessage>
      )}
      <div className="message">
        <MessageHead
          fromName={view.fromName}
          fromAddress={view.fromAddress}
          time={date}
          summary={recipientsSummary([...view.to, ...view.cc])}
          recipients={recipients}
        />
        <div className="message__body">
          {view.body.kind === "html" && <MessageFrame html={view.body.html} title={`Conteúdo da mensagem: ${subject}`} onOpenLink={openLink} />}
          {view.body.kind === "text" &&
            paragraphs(view.body.text).map((lines, i) => (
              <p key={i}>{lines.map((line, j) => <span key={j}>{j > 0 && <br />}{line}</span>)}</p>
            ))}
          {view.body.kind === "none" && <p>(mensagem sem conteúdo de texto)</p>}
        </div>
        <AttachmentList attachments={view.attachments} />
      </div>
      {compose && compose.kind !== "forward" && card}
    </Reader>
  );
}

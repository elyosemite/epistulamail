// Composição de DS: leitura.html (.reader, cabeçalho, destinatários, .message__body, .message__frame),
// mensagens.html (.section-message "Imagens externas bloqueadas", avisos e erros), anexos.html (.attachment--static),
// carregamento.html (skeleton da leitura), vazios.html (.empty)
import { useCallback, useEffect, useState } from "react";
import { formatFullDate, formatPeople, paragraphs, recipientsSummary } from "../../lib/format";
import { api, isAppError, isSessionLost, type AppError, type AppErrorKind, type MessageFlags, type MessageView as View } from "../../lib/tauri";
import { AttachmentList } from "../../ui/Attachment";
import { IconButton } from "../../ui/Button";
import { EmptyState } from "../../ui/EmptyState";
import { MessageFrame } from "../../ui/MessageFrame";
import { MessageHead, Reader, ReaderSkeleton, type RecipientRow } from "../../ui/Reader";
import { SectionMessage } from "../../ui/SectionMessage";
import { useToast } from "../../ui/Toast";

type State = { status: "loading" } | { status: "ready"; view: View } | { status: "error"; error: AppError };

interface MessageViewProps {
  messageId: string | null;
  /** Flags atuais da lista, para a leitura refletir mudanças feitas na linha. */
  flags: MessageFlags | null;
  onLoaded: (id: string, flags: MessageFlags) => void;
  onToggleStar: () => void;
  onMarkUnread: () => void;
  onSessionLost: (kind: AppErrorKind) => void;
  onBack: () => void;
}

export function MessageView({ messageId, flags, onLoaded, onToggleStar, onMarkUnread, onSessionLost, onBack }: MessageViewProps) {
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
      if (isSessionLost(error)) return onSessionLost(error.kind);
      setState({ status: "error", error: isAppError(error) ? error : { kind: "ImapConnection", message: "Tente de novo em instantes." } });
    }
  }, [onLoaded, onSessionLost]);

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
  const current = flags ?? view.flags;
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
    <Reader
      subject={subject}
      onBack={onBack}
      actions={
        <>
          <IconButton
            icon="star"
            label={current.flagged ? "Remover estrela" : "Marcar com estrela"}
            aria-pressed={current.flagged}
            className={current.flagged ? "is-selected" : undefined}
            onClick={onToggleStar}
          />
          <IconButton icon="mail" label="Marcar como não lida" onClick={onMarkUnread} disabled={!current.seen} />
        </>
      }
    >
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
    </Reader>
  );
}

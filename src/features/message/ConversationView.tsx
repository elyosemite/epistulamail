// Composição de DS: leitura.html "Conversa em cards" (US8, CA6: .conversation, .message--card, .message--collapsed,
// .conversation__more, .message__quote-toggle + .message__quote), leitura.html (.message__head, .message__body,
// .message__frame), mensagens.html (.section-message "Imagens externas bloqueadas", erros), anexos.html, carregamento.html
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { accountInitial, avatarColorClass } from "../../lib/accounts";
import { conversationLayout, initiallyOpen, revealQuote, splitQuote } from "../../lib/conversation";
import { formatFullDate, formatListDate, formatPeople, paragraphs, recipientsSummary, senderLabel } from "../../lib/format";
import { api, isAccessLost, isAppError, type Account, type AppError, type ConversationMessage, type MessageFlags, type MessageView } from "../../lib/tauri";
import { AccountAvatar } from "../../ui/AccountAvatar";
import { AttachmentList } from "../../ui/Attachment";
import { Avatar } from "../../ui/Avatar";
import { CollapsedMessage, Conversation, MessageCard, MoreMessages, QuoteToggle } from "../../ui/Conversation";
import { MessageFrame } from "../../ui/MessageFrame";
import { MessageHead, type RecipientRow } from "../../ui/Reader";
import { SectionMessage } from "../../ui/SectionMessage";

const FALLBACK: AppError = { kind: "ImapConnection", message: "Tente de novo em instantes." };

interface ConversationViewProps {
  account: Account;
  /** A mensagem escolhida na lista. */
  selectedId: string;
  messages: ConversationMessage[];
  /** Corpo carregado da mensagem escolhida (para o encaminhamento usar os anexos). */
  onSelectedView: (view: MessageView) => void;
  onLoaded: (id: string, flags: MessageFlags) => void;
  onAccessLost: () => void;
  onOpenLink: (url: string, text: string) => void;
}

/** As mensagens da conversa, da mais antiga para a mais nova, em cards (D2 e D3). */
export function ConversationView({ account, selectedId, messages, onSelectedView, onLoaded, onAccessLost, onOpenLink }: ConversationViewProps) {
  const [open, setOpen] = useState(() => initiallyOpen(messages, selectedId));
  const [showAll, setShowAll] = useState(false);
  const parts = useMemo(() => conversationLayout(messages, open, showAll), [messages, open, showAll]);
  const expand = (id: string) => setOpen((current) => new Set(current).add(id));

  // As recolhidas já vão sendo baixadas, para abrir na hora quando o usuário clicar (US9, CA3).
  useEffect(() => {
    const closed = messages.filter((m) => !open.has(m.id)).map((m) => m.id);
    if (closed.length > 0) void api.prefetchMessages(closed).catch(() => undefined);
    // Só ao carregar a conversa.
  }, [messages]);

  const avatarOf = (m: ConversationMessage, size: "sm" | "lg") =>
    m.mine ? (
      <AccountAvatar initial={accountInitial(account.displayName)} colorClass={avatarColorClass(account.color)} photo={account.photo} size={size} />
    ) : (
      <Avatar name={senderLabel(m.fromName, m.fromAddress)} seed={m.fromAddress} size={size} />
    );

  return (
    <Conversation>
      {parts.map((part) =>
        part.kind === "more" ? (
          <MoreMessages key="more" count={part.count} onClick={() => setShowAll(true)} />
        ) : part.kind === "collapsed" ? (
          <CollapsedMessage
            key={part.message.id}
            avatar={avatarOf(part.message, "sm")}
            name={part.message.mine ? "Você" : senderLabel(part.message.fromName, part.message.fromAddress)}
            snippet={part.message.snippet ?? ""}
            time={formatListDate(part.message.date)}
            unread={!part.message.flags.seen}
            onExpand={() => expand(part.message.id)}
          />
        ) : (
          <ConversationCard
            key={part.message.id}
            message={part.message}
            avatar={part.message.mine ? avatarOf(part.message, "lg") : undefined}
            onView={part.message.id === selectedId ? onSelectedView : undefined}
            onLoaded={onLoaded}
            onAccessLost={onAccessLost}
            onOpenLink={onOpenLink}
          />
        ),
      )}
    </Conversation>
  );
}

type CardState = { status: "loading" } | { status: "ready"; view: MessageView } | { status: "error"; error: AppError };

interface ConversationCardProps {
  message: ConversationMessage;
  avatar?: ReactNode;
  onView?: (view: MessageView) => void;
  onLoaded: (id: string, flags: MessageFlags) => void;
  onAccessLost: () => void;
  onOpenLink: (url: string, text: string) => void;
}

/** Uma mensagem aberta: o corpo vem de `get_message` (que também a marca como lida). */
function ConversationCard({ message, avatar, onView, onLoaded, onAccessLost, onOpenLink }: ConversationCardProps) {
  const [state, setState] = useState<CardState>({ status: "loading" });
  const [allowRemote, setAllowRemote] = useState(false);
  const [quoteOpen, setQuoteOpen] = useState(false);
  // Callbacks do pai em refs: mudam de identidade a cada render e não podem recarregar a mensagem.
  const callbacks = useRef({ onView, onLoaded, onAccessLost });
  callbacks.current = { onView, onLoaded, onAccessLost };

  const load = useCallback(async (remote: boolean, signal: { cancelled: boolean }) => {
    try {
      const view = await api.getMessage(message.id, remote);
      if (signal.cancelled) return;
      setState({ status: "ready", view });
      callbacks.current.onView?.(view);
      callbacks.current.onLoaded(message.id, view.flags);
    } catch (error) {
      if (signal.cancelled) return;
      if (isAccessLost(error)) callbacks.current.onAccessLost();
      setState({ status: "error", error: isAppError(error) ? error : FALLBACK });
    }
  }, [message.id]);

  useEffect(() => {
    const signal = { cancelled: false };
    void load(false, signal);
    return () => { signal.cancelled = true; };
  }, [load]);

  const unread = !message.flags.seen;
  const name = senderLabel(message.fromName, message.fromAddress);
  const label = `Mensagem de ${name}${unread ? ", não lida" : ""}`;
  const retry = () => {
    setState({ status: "loading" });
    void load(allowRemote, { cancelled: false });
  };

  if (state.status !== "ready") {
    return (
      <MessageCard label={label}>
        <MessageHead fromName={message.fromName} fromAddress={message.fromAddress} time={formatFullDate(message.date)} avatar={avatar} />
        {state.status === "loading" ? (
          <div style={{ display: "grid", gap: "var(--space-150)", maxWidth: "var(--layout-reading-measure)" }} aria-busy="true" aria-label="Carregando mensagem">
            {["95%", "88%", "55%"].map((w) => <span key={w} className="skeleton skeleton--line" style={{ width: w }} />)}
          </div>
        ) : state.error.kind === "MessageMoving" ? (
          <SectionMessage title="Esta mensagem está sendo movida" actions={[{ label: "Tentar de novo", onClick: retry }]}>
            Ela abre de novo em instantes, assim que a pasta de destino for atualizada.
          </SectionMessage>
        ) : state.error.kind === "MessageNotFound" ? (
          <SectionMessage appearance="warning" title="Esta mensagem não está mais nesta pasta">
            Ela pode ter sido movida ou excluída em outro dispositivo. Atualize a lista.
          </SectionMessage>
        ) : (
          <SectionMessage
            appearance="error"
            title="Não foi possível abrir a mensagem"
            actions={state.error.kind === "MessageTooLarge" ? undefined : [{ label: "Tentar de novo", onClick: retry }]}
          >
            {state.error.message}
          </SectionMessage>
        )}
      </MessageCard>
    );
  }

  const { view } = state;
  const subject = view.subject || "(sem assunto)";
  const date = formatFullDate(view.date);
  const recipients: RecipientRow[] = [
    { label: "De", value: formatPeople([{ name: view.fromName, address: view.fromAddress }]) },
    ...(view.to.length ? [{ label: "Para", value: formatPeople(view.to) }] : []),
    ...(view.cc.length ? [{ label: "Cc", value: formatPeople(view.cc) }] : []),
    { label: "Data", value: date },
  ];
  const text = view.body.kind === "text" ? splitQuote(view.body.text) : null;
  const hasQuote = view.body.kind === "html" ? view.hasQuote : !!text?.quote;
  const showImages = () => {
    setAllowRemote(true);
    void load(true, { cancelled: false });
  };

  return (
    <MessageCard label={label}>
      <MessageHead
        fromName={view.fromName}
        fromAddress={view.fromAddress}
        time={date}
        summary={recipientsSummary([...view.to, ...view.cc])}
        recipients={recipients}
        avatar={avatar}
      />
      {view.blockedRemoteImages > 0 && !allowRemote && (
        <SectionMessage title="Imagens externas bloqueadas" actions={[{ label: "Mostrar imagens", onClick: showImages }]}>
          Para proteger sua privacidade, {view.blockedRemoteImages === 1 ? "1 imagem desta mensagem não foi carregada" : `${view.blockedRemoteImages} imagens desta mensagem não foram carregadas`}.
        </SectionMessage>
      )}
      <div className="message__body">
        {view.body.kind === "html" && (
          <MessageFrame html={quoteOpen ? revealQuote(view.body.html) : view.body.html} title={`Conteúdo da mensagem: ${subject}`} onOpenLink={onOpenLink} />
        )}
        {text && paragraphs(text.own).map((lines, i) => (
          <p key={i}>{lines.map((line, j) => <span key={j}>{j > 0 && <br />}{line}</span>)}</p>
        ))}
        {view.body.kind === "none" && <p>(mensagem sem conteúdo de texto)</p>}
      </div>
      {hasQuote && <QuoteToggle expanded={quoteOpen} onToggle={() => setQuoteOpen((v) => !v)} />}
      {quoteOpen && text?.quote && (
        <div className="message__quote"><p>{text.quote}</p></div>
      )}
      <AttachmentList attachments={view.attachments} />
    </MessageCard>
  );
}

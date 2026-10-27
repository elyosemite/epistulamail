// Composição de DS: leitura.html (.reader, .reader__head; a conversa em cards fica no ConversationView, US8), mensagens.html
// (.section-message de erro), carregamento.html (skeleton da leitura), vazios.html (.empty), composicao.html (.compose--inline:
// resposta abaixo da conversa, encaminhamento acima), menus.html (.lozenge de prazo em .reader__labels)
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { api, isAccessLost, isAppError, type Account, type AppError, type ConversationMessage, type DraftKind, type MessageFlags, type MessageView as View, type MisleadingLink } from "../../lib/tauri";
import { EmptyState } from "../../ui/EmptyState";
import { Reader, ReaderSkeleton } from "../../ui/Reader";
import { SectionMessage } from "../../ui/SectionMessage";
import { useToast } from "../../ui/Toast";
import { ComposeCard } from "./ComposeCard";
import { ConversationView } from "./ConversationView";
import { LinkCheckDialog } from "./LinkCheckDialog";

/** Card de resposta ou encaminhamento aberto no painel de leitura. */
export interface ComposeState {
  kind: DraftKind;
  messageId: string | null;
}

type State = { status: "loading" } | { status: "ready"; messages: ConversationMessage[] } | { status: "error"; error: AppError };

interface MessageViewProps {
  account: Account;
  messageId: string | null;
  /** Prazo da sinalização da mensagem aberta (lozenge), vindo da lista. */
  badge?: ReactNode;
  compose: ComposeState | null;
  onCloseCompose: () => void;
  /** Envio agendado pelo card (US7): o Mailbox fecha o card e mostra o "Desfazer". */
  onSent: (sendId: string) => void;
  onLoaded: (id: string, flags: MessageFlags) => void;
  onAccessLost: () => void;
  onBack: () => void;
  /** Gerenciador de chaves aberto da leitura (US13). */
  onManageKeys: (fingerprint: string | null, technology?: "openpgp" | "smime") => void;
}

/** A conversa da mensagem escolhida na lista (US8, CA6), com a resposta ou o encaminhamento. */
export function MessageView({ account, messageId, badge, compose, onCloseCompose, onSent, onLoaded, onAccessLost, onBack, onManageKeys }: MessageViewProps) {
  const [state, setState] = useState<State>({ status: "loading" });
  const [selectedView, setSelectedView] = useState<View | null>(null);
  const toast = useToast();

  const load = useCallback(async (id: string, signal: { cancelled: boolean }) => {
    setState({ status: "loading" });
    setSelectedView(null);
    try {
      const messages = await api.getConversation(id);
      if (signal.cancelled) return;
      setState({ status: "ready", messages });
    } catch (error) {
      if (signal.cancelled) return;
      if (isAccessLost(error)) onAccessLost();
      setState({ status: "error", error: isAppError(error) ? error : { kind: "ImapConnection", message: "Tente de novo em instantes." } });
    }
  }, [onAccessLost]);

  useEffect(() => {
    if (!messageId) return;
    const signal = { cancelled: false };
    void load(messageId, signal);
    return () => { signal.cancelled = true; };
  }, [messageId, load]);

  // Link que leva a outro domínio espera a confirmação (US10, CA8).
  const [misleading, setMisleading] = useState<{ url: string; text: string; warning: MisleadingLink } | null>(null);
  const openLink = useCallback((url: string, text: string, confirmed = false) => {
    api.openExternalLink(url, text, confirmed)
      .then((result) => {
        if (result.warning) setMisleading({ url, text, warning: result.warning });
      })
      .catch(() =>
        toast({ kind: "warning", title: "Este link não pode ser aberto", description: "Só links da web e de e-mail saem do EpistulaMail." }),
      );
  }, [toast]);
  const linkDialog = misleading && (
    <LinkCheckDialog
      link={misleading.warning}
      onOpen={() => {
        openLink(misleading.url, misleading.text, true);
        setMisleading(null);
      }}
      onClose={() => setMisleading(null)}
    />
  );

  if (compose?.kind === "new") {
    return (
      <div className="reader reader--fluid">
        <ComposeCard key="new" account={account} kind="new" messageId={null} onClose={onCloseCompose} onSent={onSent} />
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
        <div className="reader reader--fluid">
          <SectionMessage appearance="warning" title="Esta mensagem não está mais nesta pasta">
            Ela pode ter sido movida ou excluída em outro dispositivo. Atualize a lista.
          </SectionMessage>
        </div>
      );
    }
    return (
      <div className="reader reader--fluid">
        <SectionMessage appearance="error" title="Não foi possível abrir a conversa" actions={[{ label: "Tentar de novo", onClick: () => void load(messageId, { cancelled: false }) }]}>
          {state.error.message}
        </SectionMessage>
      </div>
    );
  }

  const selected = state.messages.find((m) => m.id === messageId) ?? state.messages[state.messages.length - 1];
  // O encaminhamento leva os anexos da original: espera o corpo dela carregar.
  const card = compose && compose.messageId === messageId && (compose.kind !== "forward" || selectedView) && (
    <ComposeCard
      key={`${compose.kind}-${messageId}`}
      account={account}
      kind={compose.kind}
      messageId={messageId}
      originalAttachments={selectedView?.attachments ?? []}
      replyToEncrypted={!!selectedView?.security?.encrypted}
      onClose={onCloseCompose}
      onSent={onSent}
    />
  );

  return (
    <Reader subject={selected?.subject || "(sem assunto)"} labels={badge} onBack={onBack}>
      {compose?.kind === "forward" && card}
      <ConversationView
        key={messageId}
        account={account}
        selectedId={messageId}
        messages={state.messages}
        onSelectedView={setSelectedView}
        onLoaded={onLoaded}
        onAccessLost={onAccessLost}
        onOpenLink={openLink}
        onManageKeys={onManageKeys}
      />
      {compose && compose.kind !== "forward" && card}
      {linkDialog}
    </Reader>
  );
}

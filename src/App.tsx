// DS: espacamento.html · .app-shell--no-sidebar (sem pastas nesta fase), __banner, __center, __list, __reader, .has-reader
import { useCallback, useEffect, useState } from "react";
import { ConnectAccount } from "./features/account/ConnectAccount";
import { InboxList, type InboxState } from "./features/inbox/InboxList";
import { MessageDetail } from "./features/inbox/MessageDetail";
import { api, isAppError, type Account, type AppErrorKind } from "./lib/tauri";
import { AppBar } from "./ui/AppBar";
import { Avatar } from "./ui/Avatar";
import { Banner } from "./ui/Banner";
import { Spinner } from "./ui/Spinner";

type Session = { status: "booting" } | { status: "signed-out"; notice?: string } | { status: "signed-in"; account: Account };

const NOTICE: Partial<Record<AppErrorKind, string>> = {
  TokenExpired: "Sua sessão expirou. Conecte a conta de novo para continuar.",
  ImapAuth: "O Gmail recusou o acesso desta conta. Conecte de novo para continuar.",
};

export function App() {
  const [session, setSession] = useState<Session>({ status: "booting" });

  useEffect(() => {
    api.getAccount()
      .then((account) => setSession(account ? { status: "signed-in", account } : { status: "signed-out" }))
      .catch(() => setSession({ status: "signed-out" }));
  }, []);

  const [readerOpen, setReaderOpen] = useState(false);

  const onExpired = useCallback((kind: AppErrorKind) => {
    setReaderOpen(false);
    setSession({ status: "signed-out", notice: NOTICE[kind] });
  }, []);

  const shellCls = ["app-shell app-shell--no-sidebar", readerOpen && "has-reader"].filter(Boolean).join(" ");

  return (
    <div className={shellCls}>
      <AppBar actions={session.status === "signed-in" && <AccountIndicator account={session.account} />} />
      {session.status === "signed-out" && session.notice && (
        <div className="app-shell__banner"><Banner appearance="error">{session.notice}</Banner></div>
      )}
      {session.status === "booting" && (
        <div className="app-shell__center"><Spinner size="lg" label="Carregando o Epistula" /></div>
      )}
      {session.status === "signed-out" && (
        <div className="app-shell__center">
          <ConnectAccount onConnected={(account) => setSession({ status: "signed-in", account })} />
        </div>
      )}
      {session.status === "signed-in" && <Inbox key={session.account.email} onExpired={onExpired} onReaderOpenChange={setReaderOpen} />}
    </div>
  );
}

/** DS: header.html "Conta conectada" · .appbar__account */
function AccountIndicator({ account }: { account: Account }) {
  return (
    <div className="appbar__account" role="status">
      <Avatar name={account.email} seed={account.email} size="sm" />
      <span className="appbar__account-text">Conectado como <span className="appbar__account-email">{account.email}</span></span>
    </div>
  );
}

interface InboxProps {
  onExpired: (kind: AppErrorKind) => void;
  onReaderOpenChange: (open: boolean) => void;
}

function Inbox({ onExpired, onReaderOpenChange }: InboxProps) {
  const [state, setState] = useState<InboxState>({ status: "loading" });
  const [selectedUid, setSelectedUid] = useState<number | null>(null);

  const select = useCallback((uid: number | null) => {
    setSelectedUid(uid);
    onReaderOpenChange(uid !== null);
  }, [onReaderOpenChange]);

  const load = useCallback(async () => {
    setState({ status: "loading" });
    try {
      setState({ status: "ready", messages: await api.listInbox() });
    } catch (error) {
      if (isAppError(error) && (error.kind === "TokenExpired" || error.kind === "ImapAuth")) return onExpired(error.kind);
      setState({ status: "error", error: isAppError(error) ? error : { kind: "ImapConnection", message: "Tente de novo em instantes." } });
    }
  }, [onExpired]);

  useEffect(() => { void load(); }, [load]);

  const selected = state.status === "ready" ? state.messages.find((m) => m.uid === selectedUid) ?? null : null;

  return (
    <>
      <section className="app-shell__list" aria-label="Caixa de entrada">
        <InboxList state={state} selectedUid={selectedUid} onSelect={select} onRefresh={load} />
      </section>
      <section className="app-shell__reader" aria-label="Mensagem">
        <MessageDetail message={selected} onBack={() => select(null)} />
      </section>
    </>
  );
}

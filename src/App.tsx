// DS: espacamento.html · .app-shell (sidebar + lista + leitura), __banner, __center, .has-reader
import { useCallback, useEffect, useState } from "react";
import { ConnectAccount } from "./features/account/ConnectAccount";
import { Mailbox } from "./features/mailbox/Mailbox";
import { api, type Account, type AppErrorKind } from "./lib/tauri";
import { AppBar } from "./ui/AppBar";
import { Avatar } from "./ui/Avatar";
import { Banner } from "./ui/Banner";
import { Spinner } from "./ui/Spinner";
import { ToastProvider } from "./ui/Toast";

type Session = { status: "booting" } | { status: "signed-out"; notice?: string } | { status: "signed-in"; account: Account };

const NOTICE: Partial<Record<AppErrorKind, string>> = {
  TokenExpired: "Sua sessão expirou. Conecte a conta de novo para continuar.",
  ImapAuth: "O Gmail recusou o acesso desta conta. Conecte de novo para continuar.",
};

export function App() {
  const [session, setSession] = useState<Session>({ status: "booting" });
  const [readerOpen, setReaderOpen] = useState(false);

  useEffect(() => {
    api.getAccount()
      .then((account) => setSession(account ? { status: "signed-in", account } : { status: "signed-out" }))
      .catch(() => setSession({ status: "signed-out" }));
  }, []);

  const onSessionLost = useCallback((kind: AppErrorKind) => {
    setReaderOpen(false);
    setSession({ status: "signed-out", notice: NOTICE[kind] });
  }, []);

  const signedIn = session.status === "signed-in";
  const shellCls = ["app-shell", !signedIn && "app-shell--no-sidebar", readerOpen && "has-reader"].filter(Boolean).join(" ");

  return (
    <ToastProvider>
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
        {session.status === "signed-in" && (
          <Mailbox key={session.account.email} onSessionLost={onSessionLost} onReaderOpenChange={setReaderOpen} />
        )}
      </div>
    </ToastProvider>
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

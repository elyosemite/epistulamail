// DS: espacamento.html · .app-shell (--with-rail com a coluna de contas, --commands com a barra de comandos,
//     --no-sidebar sem contas), __banner, __center, __resizer, .has-reader; header.html · .appbar--shell com a busca
//     sobre a lista e .appbar__account com .appbar__account-name
import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { AccountSettingsDialog, SignOutDialog } from "./features/accounts/AccountDialogs";
import { AccountsRail } from "./features/accounts/AccountsRail";
import { AddAccount, ConnectFlow } from "./features/accounts/AddAccount";
import { Mailbox } from "./features/mailbox/Mailbox";
import { accountInitial, avatarColorClass, pickActive } from "./lib/accounts";
import { LIST_DEFAULT } from "./lib/listWidth";
import { api, onBackendEvent, type Account, type Connected } from "./lib/tauri";
import { AppBar } from "./ui/AppBar";
import { Banner } from "./ui/Banner";
import { Resizer } from "./ui/Resizer";
import { SearchField } from "./ui/SearchField";
import { Spinner } from "./ui/Spinner";
import { ToastProvider, useToast } from "./ui/Toast";

type View = { kind: "mail" } | { kind: "add" } | { kind: "reconnect"; account: Account };
type Dialog = { kind: "settings"; account: Account; isNew: boolean } | { kind: "sign-out"; account: Account } | null;

export function App() {
  return (
    <ToastProvider>
      <Shell />
    </ToastProvider>
  );
}

function Shell() {
  const [accounts, setAccounts] = useState<Account[] | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [view, setView] = useState<View>({ kind: "mail" });
  const [dialog, setDialog] = useState<Dialog>(null);
  const [readerOpen, setReaderOpen] = useState(false);
  const [offline, setOffline] = useState<Record<string, boolean>>({});
  const [loadError, setLoadError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [listWidth, setListWidth] = useState(LIST_DEFAULT);
  const searchRef = useRef<HTMLInputElement>(null);
  // Pasta aberta por conta nesta sessão, para voltar ao mesmo lugar ao trocar de conta.
  const lastFolder = useRef(new Map<string, string>());
  const toast = useToast();

  const reload = useCallback(async () => {
    try {
      const list = await api.listAccounts();
      setAccounts(list);
      setActiveId((current) => pickActive(list, current));
      setLoadError(null);
    } catch (error) {
      setAccounts((current) => current ?? []);
      setLoadError(error && typeof error === "object" && "message" in error ? String(error.message) : "Não foi possível carregar as contas.");
    }
  }, []);

  useEffect(() => {
    void reload();
    const offs = [
      onBackendEvent("accounts-updated", () => void reload()),
      onBackendEvent("account-status", () => void reload()),
      onBackendEvent("sync-status", ({ accountId, online }) => setOffline((s) => (s[accountId] === !online ? s : { ...s, [accountId]: !online }))),
    ];
    return () => offs.forEach((off) => off());
  }, [reload]);

  const select = useCallback((id: string) => {
    setActiveId(id);
    setQuery("");
    setView({ kind: "mail" });
    setReaderOpen(false);
  }, []);

  const onConnected = useCallback(
    async (result: Connected) => {
      await reload();
      select(result.account.id);
      if (result.isNew) setDialog({ kind: "settings", account: result.account, isNew: true });
      else if (result.wasActive) toast({ kind: "info", title: "Esta conta já está no Epistula" });
      else toast({ kind: "success", title: `Você entrou de novo em ${result.account.displayName}` });
    },
    [reload, select, toast],
  );

  const onFolderChange = useCallback((accountId: string, folderId: string) => {
    lastFolder.current.set(accountId, folderId);
    void api.setLastFolder(folderId).catch(() => undefined);
  }, []);
  const onAccessLost = useCallback(() => void reload(), [reload]);

  const move = useCallback(async (account: Account, direction: -1 | 1) => {
    try {
      setAccounts(await api.moveAccount(account.id, direction));
    } catch {
      toast({ kind: "error", title: "Não foi possível mudar a ordem das contas" });
    }
  }, [toast]);

  // Largura da lista guardada por conta.
  const activeAccount = accounts?.find((a) => a.id === activeId) ?? accounts?.[0];
  const savedWidth = activeAccount?.listWidth ?? LIST_DEFAULT;
  useEffect(() => setListWidth(savedWidth), [activeAccount?.id, savedWidth]);

  // "/" ou Ctrl+E levam à busca.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && (e.target.isContentEditable || ["INPUT", "TEXTAREA"].includes(e.target.tagName));
      if ((e.key === "/" && !typing && !e.ctrlKey && !e.metaKey) || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "e")) {
        if (!searchRef.current) return;
        e.preventDefault();
        searchRef.current.focus();
        searchRef.current.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (accounts === null) {
    return (
      <div className="app-shell app-shell--no-sidebar">
        <AppBar />
        <div className="app-shell__center"><Spinner size="lg" label="Carregando o Epistula" /></div>
      </div>
    );
  }

  if (accounts.length === 0) {
    return (
      <div className="app-shell app-shell--no-sidebar">
        <AppBar />
        {loadError && <div className="app-shell__banner"><Banner appearance="error">{loadError}</Banner></div>}
        <div className="app-shell__center">
          <AddAccount first onConnected={onConnected} onCancel={() => undefined} />
        </div>
      </div>
    );
  }

  const active = accounts.find((a) => a.id === activeId) ?? accounts[0];
  const showMail = view.kind === "mail";
  const shellCls = ["app-shell", "app-shell--with-rail", showMail && "app-shell--commands", showMail && readerOpen && "has-reader"].filter(Boolean).join(" ");

  return (
    <div className={shellCls} style={{ "--app-shell-list": `${listWidth}px` } as CSSProperties}>
      <AppBar
        actions={<ActiveAccount account={active} />}
        search={
          showMail && (
            <SearchField
              ref={searchRef}
              value={query}
              placeholder={`Buscar em ${active.displayName}`}
              label="Buscar e-mails nas mensagens salvas"
              shortcut="/"
              onChange={setQuery}
              onEscape={() => {
                setQuery("");
                searchRef.current?.blur();
              }}
            />
          )
        }
      />
      {offline[active.id] && active.status === "active" && (
        <div className="app-shell__banner"><Banner appearance="warning">Sem conexão. Mostrando mensagens salvas neste computador.</Banner></div>
      )}
      <AccountsRail
        accounts={accounts}
        activeId={showMail ? active.id : null}
        onSelect={select}
        onAdd={() => setView({ kind: "add" })}
        onEdit={(account) => setDialog({ kind: "settings", account, isNew: false })}
        onMove={move}
        onSignOut={(account) => setDialog({ kind: "sign-out", account })}
        onReconnect={(account) => setView({ kind: "reconnect", account })}
      />
      {view.kind === "add" && (
        <div className="app-shell__center">
          <AddAccount first={false} onConnected={onConnected} onCancel={() => setView({ kind: "mail" })} />
        </div>
      )}
      {view.kind === "reconnect" && (
        <div className="app-shell__center">
          <ConnectFlow
            providerName="Google"
            run={() => api.reconnectAccount(view.account.id)}
            onConnected={onConnected}
            onBack={() => setView({ kind: "mail" })}
          />
        </div>
      )}
      {showMail && (
        <Mailbox
          key={active.id}
          account={active}
          query={query}
          initialFolderId={lastFolder.current.get(active.id) ?? active.lastFolderId}
          onFolderChange={(folderId) => onFolderChange(active.id, folderId)}
          onAccessLost={onAccessLost}
          onReconnect={() => setView({ kind: "reconnect", account: active })}
          onReaderOpenChange={setReaderOpen}
        />
      )}
      {showMail && (
        <Resizer
          label="Largura da lista de mensagens"
          width={listWidth}
          onChange={setListWidth}
          onCommit={(width) => void api.setListWidth(active.id, width).then(() => reload()).catch(() => undefined)}
        />
      )}
      {dialog?.kind === "settings" && (
        <AccountSettingsDialog
          account={dialog.account}
          isNew={dialog.isNew}
          onSave={async (change) => {
            await api.updateAccount(dialog.account.id, change);
            await reload();
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "sign-out" && (
        <SignOutDialog
          account={dialog.account}
          onConfirm={async () => {
            try {
              await api.signOutAccount(dialog.account.id);
              toast({ kind: "info", title: `Você saiu de ${dialog.account.displayName}`, description: "As mensagens salvas continuam neste computador." });
            } catch {
              toast({ kind: "error", title: "Não foi possível sair da conta", description: "Tente de novo em instantes." });
            }
            await reload();
          }}
          onClose={() => setDialog(null)}
        />
      )}
    </div>
  );
}

/** DS: header.html "Conta conectada" · .appbar__account, .appbar__account-name */
function ActiveAccount({ account }: { account: Account }) {
  const avatar = ["avatar avatar--sm", avatarColorClass(account.color)].filter(Boolean).join(" ");
  return (
    <div className="appbar__account" role="status">
      <span className={avatar} aria-hidden="true">{accountInitial(account.displayName)}</span>
      <span className="appbar__account-text">
        <span className="appbar__account-name">{account.displayName}</span> · {account.email}
      </span>
    </div>
  );
}

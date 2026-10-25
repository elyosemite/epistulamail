// Layout (US5): AppLayout em blocos com AccountsBar, Header (marca, busca, configurações, conta e comandos),
// NavigationBlock e ContentColumn (montados pelo Mailbox). DS: espacamento.html (.app-shell--blocks, __banner,
// __center, __resizer, .has-reader; --no-sidebar antes da primeira conta), modais.html (configurações)
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { AccountSettingsDialog, SignOutDialog } from "./features/accounts/AccountDialogs";
import { AddAccount, ConnectFlow } from "./features/accounts/AddAccount";
import { Mailbox } from "./features/mailbox/Mailbox";
import { Settings } from "./features/settings/Settings";
import { AccountsBar } from "./layout/AccountsBar";
import { AppLayout } from "./layout/AppLayout";
import { AppSkeleton } from "./layout/AppSkeleton";
import { Header } from "./layout/Header";
import { HeaderAccount } from "./layout/HeaderAccount";
import { HeaderBrand } from "./layout/HeaderBrand";
import { HeaderSearch } from "./layout/HeaderSearch";
import { SettingsButton } from "./layout/SettingsButton";
import { pickActive } from "./lib/accounts";
import { applyAppearance, cachedAppearance, parseAppearance } from "./lib/appearance";
import { LIST_DEFAULT } from "./lib/listWidth";
import { api, onBackendEvent, type Account, type Appearance, type Connected } from "./lib/tauri";
import { AppBar } from "./ui/AppBar";
import { Banner } from "./ui/Banner";
import { Resizer } from "./ui/Resizer";
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
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [appearance, setAppearance] = useState<Appearance>(cachedAppearance);
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

  // Aparência salva no banco (US6); a cópia local já foi aplicada no main.tsx.
  useEffect(() => {
    api.getAppearance().then((saved) => {
      const parsed = parseAppearance(saved);
      setAppearance(parsed);
      applyAppearance(parsed);
    }).catch(() => undefined);
  }, []);

  const changeAppearance = useCallback((change: Partial<Appearance>) => {
    setAppearance((current) => {
      const next = { ...current, ...change };
      applyAppearance(next);
      return next;
    });
    api.setAppearance(change).catch(() => toast({ kind: "error", title: "Não foi possível salvar a aparência", description: "Ela volta ao que era na próxima abertura." }));
  }, [toast]);

  // Com a primeira tela de contas desenhada, o backend começa a sincronização automática.
  const loaded = accounts !== null;
  useEffect(() => {
    if (loaded) void api.appReady().catch(() => undefined);
  }, [loaded]);

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

  // "/" ou Ctrl+E levam à busca; Ctrl+, abre e fecha as configurações.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setSettingsOpen((open) => !open);
        return;
      }
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

  // Abertura: a tela em blocos aparece já com cada componente em carregamento (skeleton).
  if (accounts === null) return <AppSkeleton />;

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

  const renderHeader = (commands?: ReactNode) => (
    <Header
      brand={<HeaderBrand />}
      search={
        <HeaderSearch
          ref={searchRef}
          value={query}
          placeholder={`Buscar em ${active.displayName}`}
          inert={!showMail || settingsOpen}
          onChange={setQuery}
          onEscape={() => {
            setQuery("");
            searchRef.current?.blur();
          }}
          settings={<SettingsButton open={settingsOpen} onClick={() => setSettingsOpen((open) => !open)} />}
        />
      }
      account={<HeaderAccount account={active} />}
      commands={commands}
    />
  );

  return (
    <AppLayout listWidth={listWidth} hasReader={showMail && readerOpen}>
      <AccountsBar
        accounts={accounts}
        activeId={showMail ? active.id : null}
        onSelect={select}
        onAdd={() => setView({ kind: "add" })}
        onEdit={(account) => setDialog({ kind: "settings", account, isNew: false })}
        onMove={move}
        onSignOut={(account) => setDialog({ kind: "sign-out", account })}
        onReconnect={(account) => setView({ kind: "reconnect", account })}
      />
      {!showMail && renderHeader()}
      {offline[active.id] && active.status === "active" && (
        <div className="app-shell__banner"><Banner appearance="warning">Sem conexão. Mostrando mensagens salvas neste computador.</Banner></div>
      )}
      {view.kind === "add" && (
        <div className="app-shell__center app-shell__block">
          <AddAccount first={false} onConnected={onConnected} onCancel={() => setView({ kind: "mail" })} />
        </div>
      )}
      {view.kind === "reconnect" && (
        <div className="app-shell__center app-shell__block">
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
          settingsOpen={settingsOpen}
          renderHeader={renderHeader}
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
      {settingsOpen && (
        <Settings accounts={accounts} appearance={appearance} onAppearance={changeAppearance} onClose={() => setSettingsOpen(false)} />
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
    </AppLayout>
  );
}

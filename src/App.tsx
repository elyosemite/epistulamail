// Layout (US5): AppLayout em blocos com AccountsBar, Header e os blocos do Mailbox (NavigationBlock e ContentColumn).
// US7: o header tem a marca à esquerda, a busca e, na extrema direita, os botões das colunas e a foto da conta (menu
// da conta com configurações, tema, densidade e sair). DS: espacamento.html (.app-shell--blocks, __banner, __center,
// __resizer, .has-reader, --no-folders, --no-reader; --no-sidebar antes da primeira conta), modais.html (configurações)
// US8: o sino de notificações fica entre os botões das colunas e a foto (header.html, L52); clicar numa notificação, no
// Windows ou no sino, abre a conta e a mensagem; "Novo e-mail" na bandeja abre a composição.
// US10: o LockGate mostra a tela de desbloqueio (carregamento.html, L53) e desmonta o app inteiro ao bloquear.
// US15: abrir arquivos de e-mail (arquivos.html): o assistente fica fora dos dois layouts, para seguir aberto quando a
// primeira tela vira o app; arrastar um arquivo sobre a janela mostra a área de soltar (L70).
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { AccountSettingsDialog, RemoveArchiveDialog, SignOutDialog } from "./features/accounts/AccountDialogs";
import { ImportDialog } from "./features/import/ImportDialog";
import { AddAccount, ConnectFlow } from "./features/accounts/AddAccount";
import { LockGate, useAppLock } from "./features/lock/LockGate";
import { Mailbox, type MailboxRequest } from "./features/mailbox/Mailbox";
import { Settings } from "./features/settings/Settings";
import { AccountsBar } from "./layout/AccountsBar";
import { AppLayout } from "./layout/AppLayout";
import { AccountMenu } from "./layout/AccountMenu";
import { AppSkeleton } from "./layout/AppSkeleton";
import { ColumnToggles } from "./layout/ColumnToggles";
import { Header } from "./layout/Header";
import { HeaderBrand } from "./layout/HeaderBrand";
import { HeaderSearch } from "./layout/HeaderSearch";
import { EMPTY_SEARCH, type Search } from "./lib/search";
import { NotificationBell } from "./layout/NotificationBell";
import { UpdateDialog } from "./features/update/UpdateDialog";
import { pickActive, visibleAccounts } from "./lib/accounts";
import { applyAppearance, cachedAppearance, parseAppearance } from "./lib/appearance";
import { LIST_DEFAULT } from "./lib/listWidth";
import { appliedText, emailsLabel } from "./lib/rules";
import { finishedToast, messagesLabel } from "./lib/importing";
import type { RulesRequest } from "./features/rules/RulesSettings";
import { api, isAppError, onBackendEvent, type Account, type Appearance, type Connected, type NoticeTarget } from "./lib/tauri";
import { AppBar } from "./ui/AppBar";
import { Banner } from "./ui/Banner";
import { DropZone } from "./ui/DropZone";
import { Resizer } from "./ui/Resizer";
import { ToastProvider, useToast } from "./ui/Toast";

type View = { kind: "mail" } | { kind: "add" } | { kind: "reconnect"; account: Account };
type Dialog = { kind: "settings"; account: Account; isNew: boolean } | { kind: "sign-out"; account: Account } | { kind: "remove-archive"; account: Account } | null;

export function App() {
  return (
    <ToastProvider>
      <LockGate>
        <Shell />
      </LockGate>
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
  const [search, setSearch] = useState<Search>(EMPTY_SEARCH);
  const [listWidth, setListWidth] = useState(LIST_DEFAULT);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState("general");
  const [rulesRequest, setRulesRequest] = useState<RulesRequest | null>(null);
  const rulesRequests = useRef(0);
  const [keysRequest, setKeysRequest] = useState<{ fingerprint: string | null; technology?: "openpgp" | "smime" } | null>(null);
  const [bellOpen, setBellOpen] = useState(false);
  const [updateOpen, setUpdateOpen] = useState(false);
  // Pedido ao Mailbox: abrir uma pasta e mensagem (notificação) ou começar um e-mail (bandeja).
  const [request, setRequest] = useState<MailboxRequest | null>(null);
  const requestCount = useRef(0);
  const [appearance, setAppearance] = useState<Appearance>(cachedAppearance);
  const [accountMenuOpen, setAccountMenuOpen] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);
  // Pasta aberta por conta nesta sessão, para voltar ao mesmo lugar ao trocar de conta.
  const lastFolder = useRef(new Map<string, string>());
  const toast = useToast();
  const appLock = useAppLock();
  // Abrir arquivo de e-mail (US15): o caminho já escolhido (arrastado) e o arquivo local que recebe mais.
  const [importer, setImporter] = useState<{ path: string | null; into: Account | null } | null>(null);
  const importerOpen = useRef(false);
  const accountsRef = useRef(accounts);
  accountsRef.current = accounts;
  importerOpen.current = importer !== null;
  const [dropping, setDropping] = useState(false);

  const reload = useCallback(async () => {
    try {
      const list = visibleAccounts(await api.listAccounts());
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

  const showReaderRef = useRef(appearance.showReader);
  showReaderRef.current = appearance.showReader;
  const onReaderOpenChange = useCallback((open: boolean) => {
    setReaderOpen(open);
    if (open && !showReaderRef.current) changeAppearance({ showReader: true });
  }, [changeAppearance]);

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
    setSearch(EMPTY_SEARCH);
    setView({ kind: "mail" });
    setReaderOpen(false);
  }, []);

  const onConnected = useCallback(
    async (result: Connected) => {
      await reload();
      select(result.account.id);
      if (result.isNew) setDialog({ kind: "settings", account: result.account, isNew: true });
      else if (result.wasActive) toast({ kind: "info", title: "Esta conta já está no EpistulaMail" });
      else toast({ kind: "success", title: `Você entrou de novo em ${result.account.displayName}` });
    },
    [reload, select, toast],
  );

  const openSettings = useCallback((section = "general") => {
    setRulesRequest(null);
    setKeysRequest(null);
    setSettingsSection(section);
    setSettingsOpen(true);
  }, []);

  // Regras (US12): o editor ou o histórico abertos de fora das Configurações.
  const openRules = useCallback((request: RulesRequest) => {
    rulesRequests.current += 1;
    setKeysRequest(null);
    setRulesRequest(request);
    setSettingsSection("rules");
    setSettingsOpen(true);
  }, []);

  // Gerenciador de chaves aberto da leitura (US13): a folha abre direto nele.
  const openKeys = useCallback((fingerprint: string | null, technology: "openpgp" | "smime" = "openpgp") => {
    rulesRequests.current += 1;
    setRulesRequest(null);
    setKeysRequest({ fingerprint, technology });
    setSettingsSection("crypto");
    setSettingsOpen(true);
  }, []);

  // Arquivo arrastado sobre a janela (US15, CA1): a área de soltar e, ao soltar, o passo 2 direto.
  useEffect(() => {
    let off: (() => void) | null = null;
    let disposed = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const e = event.payload;
        if (e.type === "enter" || e.type === "over") setDropping(!importerOpen.current);
        else if (e.type === "leave") setDropping(false);
        else if (e.type === "drop") {
          setDropping(false);
          if (!importerOpen.current && e.paths[0]) setImporter({ path: e.paths[0], into: null });
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else off = fn;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      off?.();
    };
  }, []);

  // Fim de uma importação que correu em segundo plano (o assistente fechado): um aviso com "Abrir".
  useEffect(() => onBackendEvent("import-finished", ({ archive, stopped }) => {
    void reload();
    if (importerOpen.current) return;
    const name = accountsRef.current?.find((a) => a.id === archive.accountId)?.displayName ?? archive.sourceName;
    toast({ ...finishedToast(archive, name, stopped), action: { label: "Abrir", onClick: () => select(archive.accountId) } });
  }), [reload, toast, select]);

  // Importação interrompida ao fechar o app (D5): continuar ou descartar o que entrou.
  useEffect(() => {
    if (!loaded) return;
    api.listArchives().then((archives) => {
      for (const a of archives.filter((x) => x.state === "interrupted" && !x.running)) {
        const name = accountsRef.current?.find((x) => x.id === a.accountId)?.displayName ?? a.sourceName;
        toast({
          kind: "warning",
          persist: true,
          title: `A importação de ${name} parou`,
          description: `${messagesLabel(a.imported)} entraram.`,
          actions: [
            {
              label: "Continuar",
              onClick: () => {
                api.resumeMailImport(a.accountId).catch((error) =>
                  toast({ kind: "error", title: "Não foi possível continuar", description: isAppError(error) ? error.message : undefined }),
                );
              },
            },
            {
              label: "Descartar o que entrou",
              onClick: () => {
                api.removeArchive(a.accountId).then(() => reload()).catch(() => toast({ kind: "error", title: "Não foi possível descartar" }));
              },
            },
          ],
        });
      }
    }).catch(() => undefined);
  }, [loaded, toast, reload]);

  // Aviso depois de uma sincronização em que regras agiram (CA6), com "Ver" e "Desfazer".
  useEffect(() => onBackendEvent("rules-applied", (e) => {
    toast({
      kind: "info",
      icon: "filter",
      title: `Regras organizaram ${emailsLabel(e.count)}`,
      description: appliedText(e),
      actions: [
        { label: "Ver", onClick: () => openRules({ accountId: e.accountId, history: true }) },
        {
          label: "Desfazer",
          onClick: () => {
            Promise.all(e.runIds.map((id) => api.undoRuleRun(e.accountId, id)))
              .then((n) => toast({ kind: "success", title: `${emailsLabel(n.reduce((a, b) => a + b, 0))} voltaram para onde estavam` }))
              .catch(() => toast({ kind: "error", title: "Não foi possível desfazer", description: "Tente pelo histórico das regras." }));
          },
        },
      ],
    });
  }), [toast, openRules]);

  // Clique numa notificação (do Windows ou do sino): a conta, a pasta e a mensagem; "Reconecte" abre o login.
  const openTarget = useCallback((target: NoticeTarget) => {
    const account = accountsRef.current?.find((a) => a.id === target.accountId);
    if (!account) return;
    setActiveId(account.id);
    setSearch(EMPTY_SEARCH);
    setSettingsOpen(false);
    setDialog(null);
    if (target.reconnect) return setView({ kind: "reconnect", account });
    setView({ kind: "mail" });
    setRequest({ nonce: ++requestCount.current, kind: "open", accountId: account.id, folderId: target.folderId, messageId: target.messageId });
  }, []);

  useEffect(() => {
    const offs = [
      onBackendEvent("open-target", openTarget),
      onBackendEvent("tray-compose", () => {
        setView({ kind: "mail" });
        setSettingsOpen(false);
        setRequest({ nonce: ++requestCount.current, kind: "compose" });
      }),
    ];
    return () => offs.forEach((off) => off());
  }, [openTarget]);

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

  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  const toggleFolders = useCallback(() => changeAppearance({ showFolders: !appearanceRef.current.showFolders }), [changeAppearance]);
  const toggleReader = useCallback(() => changeAppearance({ showReader: !appearanceRef.current.showReader }), [changeAppearance]);

  // "/" ou Ctrl+E levam à busca; Ctrl+, abre e fecha as configurações; Ctrl+B e Ctrl+Alt+B mostram e escondem
  // as pastas e a leitura; Alt+A abre o menu da conta (US7); Alt+N abre o sino (US8). Com um modal aberto, nenhum vale.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (document.querySelector(".blanket")) return;
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setSettingsSection("general");
        setSettingsOpen((open) => !open);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === "b") {
        e.preventDefault();
        if (e.altKey) toggleReader();
        else toggleFolders();
        return;
      }
      if (e.altKey && !e.ctrlKey && !e.metaKey && e.key.toLowerCase() === "a") {
        e.preventDefault();
        setAccountMenuOpen((open) => !open);
        return;
      }
      if (e.altKey && !e.ctrlKey && !e.metaKey && e.key.toLowerCase() === "n") {
        e.preventDefault();
        setBellOpen((open) => !open);
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
  }, [toggleFolders, toggleReader]);

  // Abertura: a tela em blocos aparece já com cada componente em carregamento (skeleton).
  if (accounts === null) return <AppSkeleton />;

  const openFile = (into: Account | null = null) => setImporter({ path: null, into });
  const overlays = (
    <>
      {dropping && <DropZone />}
      {importer && (
        <ImportDialog
          initialPath={importer.path}
          into={importer.into}
          onClose={() => setImporter(null)}
          onBackground={(account) => {
            void reload().then(() => select(account.id));
          }}
          onOpen={(accountId) => {
            void reload().then(() => select(accountId));
          }}
        />
      )}
    </>
  );

  if (accounts.length === 0) {
    return (
      <>
        <div className="app-shell app-shell--no-sidebar">
          <AppBar />
          {loadError && <div className="app-shell__banner"><Banner appearance="error">{loadError}</Banner></div>}
          <div className="app-shell__center">
            <AddAccount first onConnected={onConnected} onCancel={() => undefined} onOpenFile={() => openFile()} />
          </div>
        </div>
        {overlays}
      </>
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
          accountId={active.id}
          search={search}
          placeholder={search.allAccounts ? "Buscar em todas as contas" : `Buscar em ${active.displayName}`}
          inert={!showMail || settingsOpen}
          onChange={setSearch}
          onEscape={() => {
            setSearch(EMPTY_SEARCH);
            searchRef.current?.blur();
          }}
        />
      }
      end={
        <>
          <ColumnToggles showFolders={appearance.showFolders} showReader={appearance.showReader} onToggleFolders={toggleFolders} onToggleReader={toggleReader} />
          <NotificationBell open={bellOpen} onOpenChange={setBellOpen} onOpenTarget={openTarget} onSettings={() => openSettings("notifications")} onUpdate={() => setUpdateOpen(true)} />
          <AccountMenu
            account={active}
            appearance={appearance}
            offline={!!offline[active.id]}
            open={accountMenuOpen}
            onOpenChange={setAccountMenuOpen}
            onAppearance={changeAppearance}
            onSettings={() => openSettings()}
            onSignOut={() => setDialog({ kind: "sign-out", account: active })}
            onLock={appLock.hasPassword ? appLock.lockNow : undefined}
            onRename={() => setDialog({ kind: "settings", account: active, isNew: false })}
            onImportMore={() => openFile(active)}
            onOpenFile={() => openFile()}
            onRemove={() => setDialog({ kind: "remove-archive", account: active })}
          />
        </>
      }
      commands={commands}
    />
  );

  return (
    <>
    <AppLayout listWidth={listWidth} hasReader={showMail && readerOpen} showFolders={appearance.showFolders} showReader={appearance.showReader}>
      <AccountsBar
        accounts={accounts}
        activeId={showMail ? active.id : null}
        onSelect={select}
        onAdd={() => setView({ kind: "add" })}
        onEdit={(account) => setDialog({ kind: "settings", account, isNew: false })}
        onMove={move}
        onSignOut={(account) => setDialog({ kind: "sign-out", account })}
        onReconnect={(account) => setView({ kind: "reconnect", account })}
        onRemove={(account) => setDialog({ kind: "remove-archive", account })}
      />
      {!showMail && renderHeader()}
      {offline[active.id] && active.status === "active" && (
        <div className="app-shell__banner"><Banner appearance="warning">Sem conexão. Mostrando mensagens salvas neste computador.</Banner></div>
      )}
      {view.kind === "add" && (
        <div className="app-shell__center app-shell__block">
          <AddAccount
            first={false}
            onConnected={onConnected}
            onCancel={() => setView({ kind: "mail" })}
            onOpenFile={() => {
              setView({ kind: "mail" });
              openFile();
            }}
          />
        </div>
      )}
      {view.kind === "reconnect" && (
        <div className="app-shell__center app-shell__block">
          <ConnectFlow
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
          accounts={accounts}
          search={search}
          onSearchChange={setSearch}
          settingsOpen={settingsOpen}
          renderHeader={renderHeader}
          density={appearance.density}
          initialFolderId={lastFolder.current.get(active.id) ?? active.lastFolderId}
          onFolderChange={(folderId) => onFolderChange(active.id, folderId)}
          onAccessLost={onAccessLost}
          onReconnect={() => setView({ kind: "reconnect", account: active })}
          onReaderOpenChange={onReaderOpenChange}
          appRequest={request}
          onOpenRules={openRules}
          onManageKeys={openKeys}
        />
      )}
      {showMail && appearance.showReader && (
        <Resizer
          label="Largura da lista de mensagens"
          width={listWidth}
          onChange={setListWidth}
          onCommit={(width) => void api.setListWidth(active.id, width).then(() => reload()).catch(() => undefined)}
        />
      )}
      {settingsOpen && (
        <Settings
          key={rulesRequest || keysRequest ? `request-${rulesRequests.current}` : settingsSection}
          accounts={accounts}
          appearance={appearance}
          onAppearance={changeAppearance}
          initialSection={settingsSection}
          activeAccountId={active.id}
          rulesRequest={rulesRequest}
          keysRequest={keysRequest}
          onClose={() => setSettingsOpen(false)}
          onOpenFile={() => openFile()}
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
          onConfirm={async (erase) => {
            try {
              await api.signOutAccount(dialog.account.id, erase);
              toast(
                erase
                  ? { kind: "info", title: `Você saiu de ${dialog.account.displayName}`, description: "Os e-mails desta conta foram apagados deste computador." }
                  : { kind: "info", title: `Você saiu de ${dialog.account.displayName}`, description: "Entre de novo com o mesmo e-mail para ver as mensagens salvas." },
              );
            } catch {
              toast({ kind: "error", title: "Não foi possível sair da conta", description: "Tente de novo em instantes." });
            }
            await reload();
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "remove-archive" && (
        <RemoveArchiveDialog
          account={dialog.account}
          onConfirm={async () => {
            try {
              await api.removeArchive(dialog.account.id);
              toast({ kind: "info", title: `${dialog.account.displayName} saiu do EpistulaMail`, description: "As mensagens foram apagadas deste computador. O arquivo original continua onde estava." });
            } catch (error) {
              toast({ kind: "error", title: "Não foi possível remover", description: isAppError(error) ? error.message : "Tente de novo em instantes." });
            }
            await reload();
          }}
          onClose={() => setDialog(null)}
        />
      )}
    </AppLayout>
    {overlays}
    {updateOpen && <UpdateDialog onClose={() => setUpdateOpen(false)} />}
    </>
  );
}

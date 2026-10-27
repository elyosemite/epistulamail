// Composição de layout: Header (com a CommandBar), NavigationBlock (FoldersColumn + EmailListColumn) e ContentColumn.
// DS: toasts.html (desfazer e erros), mensagens.html (.section-message "Você saiu desta conta" / "Reconecte esta conta" /
// busca local), modais.html (confirmar exclusão definitiva na Lixeira); busca.html (US16): barra dos resultados
// (L74), conta de cada resultado (L75), "Aplicar regra…", "Salvar busca" e buscas salvas na coluna de pastas
import { QuickRuleDialog, type QuickIntent } from "../rules/RuleDialogs";
import type { RulesRequest } from "../rules/RulesSettings";
import { emailsLabel, suggestedName } from "../../lib/rules";
import { accountInitial, avatarColorClass } from "../../lib/accounts";
import { fromQuery, hasCriteria, ruleFromSearch, searchLabel, toQuery, type Search } from "../../lib/search";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { bucketLabel, flagDates, visibleBuckets } from "../../lib/flagged";
import { canReceive } from "../../lib/folders";
import { dueLabel } from "../../lib/followup";
import { adjustUnread, applyFlags, restoreFlags, snapshotFlags, unreadDelta } from "../../lib/flags";
import { currentCursor, firstPage, goBack, goForward, withNext, type Pager } from "../../lib/pagination";
import { batchTitle, commandTargets, conversations, emptySelection, masterState, prune, readActionFor, toggle, toggleAll, type Selection } from "../../lib/selection";
import {
  api,
  isAccessLost,
  isAppError,
  onBackendEvent,
  type Account,
  type Density,
  type DraftKind,
  type FlagBucket,
  type FlaggedCounts,
  type Folder,
  type FollowupAction,
  type Followup,
  type MessageFlags,
  type MessageHeader,
  type Moved,
  type RuleSuggestions,
  type SavedSearch,
} from "../../lib/tauri";
import { CommandBar } from "../../layout/CommandBar";
import { ContentColumn } from "../../layout/ContentColumn";
import { EmailListColumn, type ListState } from "../../layout/EmailListColumn";
import { FoldersColumn } from "../../layout/FoldersColumn";
import { NavigationBlock } from "../../layout/NavigationBlock";
import { SearchResultsBar } from "../../layout/SearchResultsBar";
import { AccountAvatar } from "../../ui/AccountAvatar";
import { EmptyState } from "../../ui/EmptyState";
import { ApplyRuleDialog } from "../search/ApplyRuleDialog";
import { Button } from "../../ui/Button";
import { Lozenge } from "../../ui/Lozenge";
import { Modal } from "../../ui/Modal";
import { SectionMessage } from "../../ui/SectionMessage";
import { useToast } from "../../ui/Toast";
import { MessageView, type ComposeState } from "../message/MessageView";

/** Pedido do App (US8): abrir a pasta e a mensagem de uma notificação, ou começar um e-mail pela bandeja. */
export type MailboxRequest =
  | { nonce: number; kind: "open"; accountId: string; folderId: string | null; messageId: string | null }
  | { nonce: number; kind: "compose" };

interface MailboxProps {
  account: Account;
  initialFolderId: string | null;
  /** Todas as contas: a conta de cada resultado na busca em todas (US16). */
  accounts: Account[];
  /** Busca do header (US16): com algum chip ou 2 letras, a lista mostra os resultados. */
  search: Search;
  onSearchChange: (search: Search) => void;
  /** Configurações abertas: os comandos ficam inertes (US5, D2). */
  settingsOpen: boolean;
  /** Monta o bloco do header com a barra de comandos desta caixa. */
  renderHeader: (commands: ReactNode) => ReactNode;
  /** Densidade da lista (US7, CA7). */
  density: Density;
  onFolderChange: (folderId: string) => void;
  /** O backend recusou por falta de acesso: o App recarrega as contas (status mudou). */
  onAccessLost: () => void;
  onReconnect: () => void;
  onReaderOpenChange: (open: boolean) => void;
  appRequest?: MailboxRequest | null;
  /** "Mais opções" de "Criar regra": abre o editor completo nas Configurações (US12, CA5). */
  onOpenRules: (request: RulesRequest) => void;
  /** Gerenciador de chaves OpenPGP nas Configurações (US13). */
  onManageKeys: (fingerprint: string | null, technology?: "openpgp" | "smime") => void;
}

const FALLBACK = { kind: "ImapConnection" as const, message: "Tente de novo em instantes." };
const SEARCH_DELAY_MS = 250;
/** Mensagens baixadas antes, depois da aberta: as que o usuário deve abrir em seguida (US9, CA3). */
const PREFETCH_NEXT = 3;

const TONE = { today: "moved", future: "default", overdue: "overdue", done: "success" } as const;

/** Lozenge do prazo da sinalização ("Hoje", "Amanhã", "Atrasada"…). */
export function DueBadge({ followup }: { followup: Followup | null }) {
  const label = dueLabel(followup);
  if (!label || label.tone === "none") return null;
  return <Lozenge appearance={TONE[label.tone]}>{label.text}</Lozenge>;
}

/** A pasta pedida ou, sem ela, a Caixa de entrada. */
function allFoldersOf(folders: Folder[], folderId: string | null): Folder | undefined {
  return folders.find((f) => f.id === folderId) ?? folders.find((f) => f.role === "inbox");
}

/** O que sai da lista numa ação em lote, como executar e o aviso de falha parcial. */
interface Removal {
  messages: MessageHeader[];
  title: string;
  run: () => Promise<Moved>;
  failedTitle: (count: number) => string;
}

export function Mailbox({ account, accounts, initialFolderId, search, onSearchChange, settingsOpen, renderHeader, density, onFolderChange, onAccessLost, onReconnect, onReaderOpenChange, appRequest, onOpenRules, onManageKeys }: MailboxProps) {
  const [folders, setFolders] = useState<Folder[] | null>(null);
  const [folder, setFolder] = useState<Folder | null>(null);
  const [pager, setPager] = useState<Pager>(firstPage);
  const [importing, setImporting] = useState<{ imported: number; estimate: number } | null>(null);
  const importReloaded = useRef(0);
  const [list, setList] = useState<ListState>({ status: "loading" });
  const [syncing, setSyncing] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [checked, setChecked] = useState<Selection>(emptySelection);
  const [dragging, setDragging] = useState<string[] | null>(null);
  const [dropHover, setDropHover] = useState<string | null>(null);
  // Prazo das sinalizadas aberto na lista (US6, CA6) e as quantidades da coluna de pastas.
  const [flagBucket, setFlagBucket] = useState<FlagBucket | null>(null);
  const [flagCounts, setFlagCounts] = useState<FlaggedCounts | null>(null);
  const [compose, setCompose] = useState<ComposeState | null>(null);
  const [confirmPurge, setConfirmPurge] = useState<MessageHeader[] | null>(null);
  const [searchTruncated, setSearchTruncated] = useState(false);
  const [searchReload, setSearchReload] = useState(0);
  const [saved, setSaved] = useState<SavedSearch[]>([]);
  const [saving, setSaving] = useState<{ name: string; busy: boolean; error: string | null } | null>(null);
  const [applyRule, setApplyRule] = useState<{ accountId: string; ids: string[] } | null>(null);
  // Pastas das outras contas (busca em todas): papel de cada resultado e destinos de "Mover".
  const [otherFolders, setOtherFolders] = useState<Record<string, Folder[]>>({});
  const toast = useToast();
  const request = useRef(0);
  // Leitura das mensagens atuais sem fazer callbacks estáveis dependerem da lista inteira.
  const itemsRef = useRef<MessageHeader[]>([]);
  itemsRef.current = list.status === "ready" ? list.items : [];
  const selectedRef = useRef(selectedId);
  selectedRef.current = selectedId;
  const pagerRef = useRef(pager);
  pagerRef.current = pager;
  const folderRef = useRef(folder);
  folderRef.current = folder;
  const initialRef = useRef(appRequest?.kind === "open" && appRequest.accountId === account.id && appRequest.folderId ? appRequest.folderId : initialFolderId);
  // Mensagem a abrir quando a pasta pedida terminar de carregar (notificação, US8).
  const pendingOpen = useRef<string | null>(null);
  // Mensagem aberta por uma notificação: se ela não for a linha da conversa na lista, marca a linha dela.
  const revealRef = useRef<string | null>(null);
  // Callbacks do App em refs: se mudarem de identidade a cada render do App, os efeitos de
  // carregar a pasta não podem rodar de novo (isso fechava a mensagem recém-aberta).
  const accessLostRef = useRef(onAccessLost);
  accessLostRef.current = onAccessLost;
  const folderChangeRef = useRef(onFolderChange);
  folderChangeRef.current = onFolderChange;
  const readerOpenRef = useRef(onReaderOpenChange);
  readerOpenRef.current = onReaderOpenChange;
  const accessLost = useCallback(() => accessLostRef.current(), []);
  const searching = hasCriteria(search);
  const searchQuery = useMemo(() => toQuery(search), [search]);
  const queryKey = JSON.stringify(searchQuery);
  // Busca ou sinalizadas: mensagens de várias pastas, sem paginação nem contadores da pasta.
  const crossFolder = searching || flagBucket !== null;
  const searchingRef = useRef(crossFolder);
  searchingRef.current = crossFolder;
  const flagBucketRef = useRef(flagBucket);
  flagBucketRef.current = flagBucket;
  const toastRef = useRef(toast);
  toastRef.current = toast;

  const fail = useCallback((error: unknown, onOther: () => void) => {
    if (isAccessLost(error)) accessLostRef.current();
    if (isAccessLost(error) && error.kind === "SignedOut") return;
    onOther();
  }, []);

  const loadFolders = useCallback(async () => {
    try {
      const all = await api.listFolders(account.id);
      setFolders(all);
      setFolder((current) => {
        const keep = current && all.find((f) => f.id === current.id);
        if (keep) return keep;
        return all.find((f) => f.id === initialRef.current) ?? all.find((f) => f.role === "inbox") ?? all[0] ?? null;
      });
    } catch (error) {
      fail(error, () => {
        setFolders((current) => current ?? []);
        setList({ status: "error", error: isAppError(error) ? error : FALLBACK });
      });
    }
  }, [account.id, fail]);

  useEffect(() => {
    void loadFolders();
  }, [loadFolders]);

  const select = useCallback((id: string | null) => {
    setSelectedId(id);
    if (id) {
      const items = itemsRef.current;
      const at = items.findIndex((m) => m.id === id);
      const next = at < 0 ? [] : items.slice(at + 1, at + 1 + PREFETCH_NEXT).map((m) => m.id);
      if (next.length > 0) void api.prefetchMessages(next).catch(() => undefined);
    }
    setCompose((c) => (c?.kind === "new" ? c : null));
    readerOpenRef.current(id !== null);
  }, []);

  /** `silent`: recarrega sem esqueleto e mantém a seleção (atualização vinda da sincronização). */
  const loadPage = useCallback(async (target: Folder, page: Pager, silent = false) => {
    if (searchingRef.current) return;
    const token = ++request.current;
    if (!silent) {
      setList({ status: "loading" });
      setChecked(emptySelection);
    }
    try {
      const result = await api.listMessages(target.id, currentCursor(page));
      if (token !== request.current) return;
      setPager(withNext(page, result.nextCursor));
      setList({ status: "ready", items: result.items, total: result.total });
      setChecked((sel) => prune(sel, result.items.map((m) => m.id)));
      setSyncing(result.syncing);
      if (result.unread !== null) {
        setFolders((all) => all && all.map((f) => (f.id === target.id ? { ...f, unread: result.unread } : f)));
      }
    } catch (error) {
      if (token !== request.current) return;
      if (isAppError(error) && error.kind === "CursorExpired") return void loadPage(target, firstPage());
      if (silent) return;
      fail(error, () => setList({ status: "error", error: isAppError(error) ? error : FALLBACK }));
    }
  }, [fail]);

  useEffect(() => {
    if (!folder || crossFolder) return;
    const page = firstPage();
    setPager(page);
    select(pendingOpen.current);
    pendingOpen.current = null;
    void loadPage(folder, page);
    folderChangeRef.current(folder.id);
    // Só ao trocar de pasta ou ao sair da busca e das sinalizadas: o objeto `folder` muda também com os contadores.
  }, [folder?.id, crossFolder, loadPage, select]);

  // Pedidos do App (US8): a pasta e a mensagem de uma notificação; "Novo e-mail" da bandeja. Fica depois do efeito
  // de trocar de pasta, que roda primeiro e limpa a seleção.
  const handled = useRef<number | null>(null);
  useEffect(() => {
    if (!appRequest || handled.current === appRequest.nonce) return;
    if (appRequest.kind === "compose") {
      handled.current = appRequest.nonce;
      setCompose({ kind: "new", messageId: null });
      readerOpenRef.current(true);
      return;
    }
    if (appRequest.accountId !== account.id || folders === null) return;
    handled.current = appRequest.nonce;
    const target = allFoldersOf(folders, appRequest.folderId);
    if (!target) return;
    const wasFlagged = flagBucketRef.current !== null;
    setFlagBucket(null);
    revealRef.current = appRequest.messageId;
    if (wasFlagged || searchingRef.current || folderRef.current?.id !== target.id) {
      pendingOpen.current = appRequest.messageId;
      setFolder(target);
    } else {
      select(appRequest.messageId);
    }
  }, [appRequest, account.id, folders, select]);

  // A lista mostra uma linha por conversa: a mensagem da notificação pode ser uma mais antiga da conversa.
  // Com a lista carregada, a linha da conversa dela fica marcada (a leitura mostra a conversa inteira).
  useEffect(() => {
    const wanted = revealRef.current;
    if (!wanted || selectedId !== wanted || list.status !== "ready") return;
    revealRef.current = null;
    if (list.items.some((m) => m.id === wanted)) return;
    api.getConversation(wanted)
      .then((messages) => {
        const ids = new Set(messages.map((m) => m.id));
        const row = itemsRef.current.find((m) => ids.has(m.id));
        if (row && selectedRef.current === wanted) setSelectedId(row.id);
      })
      .catch(() => undefined);
  }, [list, selectedId]);

  // Quantidades das sinalizadas por prazo, na data local de hoje.
  const loadFlagCounts = useCallback(() => {
    api.flaggedCounts(account.id, flagDates()).then(setFlagCounts).catch(() => undefined);
  }, [account.id]);
  useEffect(() => loadFlagCounts(), [loadFlagCounts]);

  // Mensagens de um prazo das sinalizadas (a busca tem prioridade).
  const [flagReload, setFlagReload] = useState(0);
  useEffect(() => {
    if (searching || !flagBucket) return;
    const token = ++request.current;
    if (flagReload === 0) {
      setList({ status: "loading" });
      setChecked(emptySelection);
      select(null);
    }
    api.listFlagged(account.id, flagBucket, flagDates())
      .then((page) => {
        if (token !== request.current) return;
        setList({ status: "ready", items: page.items, total: page.items.length });
        setChecked((sel) => prune(sel, page.items.map((m) => m.id)));
      })
      .catch((error) => {
        if (token !== request.current) return;
        setList({ status: "error", error: isAppError(error) ? error : FALLBACK });
      });
  }, [account.id, flagBucket, flagReload, searching, select]);
  /** Depois de mudar sinalizações ou mover, a lista do prazo e as quantidades acompanham. */
  const refreshFlagged = useCallback(() => {
    loadFlagCounts();
    setFlagReload((n) => n + 1);
  }, [loadFlagCounts]);

  // Busca local (US4 CA1, US16): 250 ms depois da última mudança nos chips ou no texto.
  useEffect(() => {
    if (!searching) return;
    const token = ++request.current;
    if (searchReload === 0) {
      setList({ status: "loading" });
      setChecked(emptySelection);
      select(null);
    }
    const timer = setTimeout(() => {
      api.searchMessages(account.id, JSON.parse(queryKey))
        .then((page) => {
          if (token !== request.current) return;
          setSearchTruncated(page.truncated);
          setList({ status: "ready", items: page.items, total: page.items.length });
          setChecked((sel) => prune(sel, page.items.map((m) => m.id)));
        })
        .catch((error) => {
          if (token !== request.current) return;
          setList({ status: "error", error: isAppError(error) ? error : FALLBACK });
        });
    }, searchReload === 0 ? SEARCH_DELAY_MS : 0);
    return () => clearTimeout(timer);
  }, [account.id, queryKey, searching, searchReload, select]);
  useEffect(() => setSearchReload(0), [queryKey]);

  // Buscas salvas (US16): na coluna de pastas, com as não lidas de cada uma.
  const loadSaved = useCallback(() => {
    api.listSavedSearches(account.id).then(setSaved).catch(() => undefined);
  }, [account.id]);
  useEffect(() => loadSaved(), [loadSaved]);

  // Abrir um resultado guarda a busca nas recentes (só neste computador).
  const recentKey = useRef<string | null>(null);
  useEffect(() => {
    if (!searching || !selectedId || recentKey.current === queryKey) return;
    recentKey.current = queryKey;
    void api.addRecentSearch({ label: searchLabel(search), query: searchQuery }).catch(() => undefined);
  }, [searching, selectedId, queryKey, search, searchQuery]);
  useEffect(() => {
    if (!searching) recentKey.current = null;
  }, [searching]);

  // A sincronização em segundo plano avisa quando a pasta ou a lista de pastas mudou.
  useEffect(() => {
    const offs = [
      onBackendEvent("mailbox-updated", ({ accountId, folderId }) => {
        if (accountId === account.id) {
          loadFlagCounts();
          loadSaved();
        }
        const current = folderRef.current;
        if (accountId !== account.id || !current || current.id !== folderId) return;
        void loadPage(current, pagerRef.current, true);
      }),
      onBackendEvent("folders-updated", ({ accountId }) => {
        if (accountId === account.id) void loadFolders();
      }),
      // Importação deste arquivo local (US15): o progresso na coluna e a lista acompanhando a cada mil.
      onBackendEvent("import-progress", (p) => {
        if (p.accountId !== account.id) return;
        setImporting({ imported: p.imported, estimate: p.estimate });
        if (p.imported - importReloaded.current < 1000 && importReloaded.current > 0) return;
        importReloaded.current = Math.max(p.imported, 1);
        void loadFolders();
        const current = folderRef.current;
        if (current) void loadPage(current, pagerRef.current, true);
      }),
      onBackendEvent("import-finished", ({ archive }) => {
        if (archive.accountId !== account.id) return;
        setImporting(null);
        importReloaded.current = 0;
        void loadFolders();
        // A ordem final segue a data: a lista volta para a primeira página.
        const current = folderRef.current;
        if (current) void loadPage(current, firstPage(), true);
      }),
      onBackendEvent("moves-finished", ({ accountId, failed, folderIds }) => {
        if (accountId !== account.id || failed.length === 0) return;
        toastRef.current({ kind: "error", title: `${conversations(failed.length)} não puderam ser movidas`, description: "O servidor recusou. Elas voltaram para a pasta de origem." });
        void loadFolders();
        loadFlagCounts();
        const current = folderRef.current;
        if (current && folderIds.includes(current.id)) void loadPage(current, pagerRef.current, true);
      }),
    ];
    return () => offs.forEach((off) => off());
  }, [account.id, loadFlagCounts, loadFolders, loadPage, loadSaved]);

  const refresh = useCallback(async () => {
    if (!folder) return;
    setSyncing(true);
    try {
      await api.syncFolder(folder.id);
    } catch (error) {
      fail(error, () => {
        if (isAppError(error) && error.kind === "ImapConnection") {
          toast({ kind: "warning", title: "Sem conexão", description: "Mostrando as mensagens salvas neste computador." });
        }
      });
    }
    setSyncing(false);
    void loadPage(folder, pagerRef.current, true);
  }, [fail, folder, loadPage, toast]);

  const changeFlags = useCallback(async (messages: MessageHeader[], change: Partial<MessageFlags>) => {
    if (messages.length === 0) return;
    const ids = messages.map((m) => m.id);
    const previous = snapshotFlags(itemsRef.current, ids);
    if (previous.size === 0) return;
    const delta = unreadDelta(previous, change);
    const folderId = folderRef.current?.id;
    setList((s) => (s.status === "ready" ? { ...s, items: applyFlags(s.items, ids, change) } : s));
    if (folderId && !searchingRef.current) setFolders((all) => all && adjustUnread(all, folderId, delta));
    try {
      const results = await api.setFlags(ids, change);
      const confirmed = new Map(results.map((r) => [r.id, r.flags]));
      setList((s) => (s.status === "ready" ? { ...s, items: s.items.map((m) => ({ ...m, flags: confirmed.get(m.id) ?? m.flags })) } : s));
      if (change.flagged !== undefined) refreshFlagged();
    } catch (error) {
      setList((s) => (s.status === "ready" ? { ...s, items: restoreFlags(s.items, previous) } : s));
      if (folderId && !searchingRef.current) setFolders((all) => all && adjustUnread(all, folderId, -delta));
      if (isAccessLost(error)) accessLost();
      toast({
        kind: "error",
        title: messages.length > 1 ? `Não foi possível atualizar ${conversations(messages.length)}` : "Não foi possível atualizar a mensagem",
        description: isAppError(error) ? error.message : undefined,
        action: isAccessLost(error) ? undefined : { label: "Tentar de novo", onClick: () => void changeFlags(messages, change) },
      });
    }
  }, [accessLost, refreshFlagged, toast]);

  // Abrir marca como lida no servidor; a lista e o contador acompanham sem nova chamada.
  const onLoaded = useCallback((id: string, flags: MessageFlags) => {
    const before = itemsRef.current.find((m) => m.id === id);
    const current = folderRef.current;
    if (before && !before.flags.seen && flags.seen && current && !searchingRef.current) {
      setFolders((all) => all && adjustUnread(all, current.id, -1));
    }
    setList((s) => (s.status === "ready" ? { ...s, items: s.items.map((m) => (m.id === id ? { ...m, flags } : m)) } : s));
  }, []);

  // ------------------------------------------------------------ seleção (US5, CA1)

  const items = list.status === "ready" ? list.items : [];
  const order = useMemo(() => items.map((m) => m.id), [items]);
  const onCheck = useCallback((id: string, range: boolean) => setChecked((sel) => toggle(sel, itemsRef.current.map((m) => m.id), id, range)), []);
  const onCheckAll = useCallback(() => setChecked((sel) => toggleAll(sel, itemsRef.current.map((m) => m.id))), []);
  const clearChecked = useCallback(() => setChecked(emptySelection), []);

  const selected = items.find((m) => m.id === selectedId) ?? null;
  const targets = useMemo(() => commandTargets(checked, items, selectedId), [checked, items, selectedId]);

  // "Criar regra…" a partir da mensagem (US12, CA5): as sugestões vêm do backend.
  const [quickRule, setQuickRule] = useState<{ intent: QuickIntent; suggestions: RuleSuggestions } | null>(null);
  const startRule = async (intent: QuickIntent) => {
    const message = targets.length === 1 ? targets[0] : null;
    if (!message) return;
    try {
      setQuickRule({ intent, suggestions: await api.ruleSuggestions(message.id) });
    } catch {
      toast({ kind: "error", title: "Não foi possível criar a regra", description: "Tente de novo em instantes." });
    }
  };
  const allFolders = useMemo(() => folders ?? [], [folders]);
  const sourceFolderId = crossFolder ? null : folder?.id ?? null;
  const accountIdOf = (m: MessageHeader) => m.accountId ?? account.id;
  const foldersOf = useCallback((id: string) => (id === account.id ? allFolders : otherFolders[id] ?? []), [account.id, allFolders, otherFolders]);
  const roleOf = useCallback(
    (m: MessageHeader) => (crossFolder ? foldersOf(m.accountId ?? account.id).find((f) => f.id === m.folderId) : allFolders.find((f) => f.id === folder?.id))?.role,
    [account.id, allFolders, folder?.id, crossFolder, foldersOf],
  );

  // Busca em todas as contas: as pastas das outras contas, para o papel de cada resultado (Lixeira pede confirmação).
  useEffect(() => {
    const missing = [...new Set(items.map((m) => m.accountId ?? account.id))].filter((id) => id !== account.id && !otherFolders[id]);
    for (const id of missing) {
      api.listFolders(id).then((all) => setOtherFolders((map) => ({ ...map, [id]: all }))).catch(() => setOtherFolders((map) => ({ ...map, [id]: [] })));
    }
  }, [items, account.id, otherFolders]);

  // Ações em lote agem numa conta só (o backend recusa misturar): a conta dos alvos, ou `null` se forem de várias.
  const targetAccounts = [...new Set(targets.map(accountIdOf))];
  const targetAccount = targetAccounts.length === 1 ? accounts.find((a) => a.id === targetAccounts[0]) ?? account : targets.length === 0 ? account : null;
  const accountOf = (m: MessageHeader | null) => (m ? accounts.find((a) => a.id === accountIdOf(m)) ?? account : account);

  // ------------------------------------------------------------ excluir, arquivar, mover em lote (US4 CA4–CA5, US5 CA2–CA3)

  /**
   * Tira as mensagens da lista na hora e registra o mover no backend, que também as tira da origem e
   * as põe no destino no banco local (US7, D1). O servidor só recebe o MOVE depois do prazo do
   * "Desfazer", e quem cuida disso é o backend: recarregar a lista não as traz de volta.
   */
  const removeWithUndo = useCallback(({ messages, title, run, failedTitle }: Removal) => {
    const current = itemsRef.current;
    const gone = new Set(messages.map((m) => m.id));
    const positions = current.flatMap((m, index) => (gone.has(m.id) ? [{ message: m, index }] : []));
    if (positions.length === 0) return;
    const after = current.slice(positions[positions.length - 1].index + 1).find((m) => !gone.has(m.id));
    const before = current.slice(0, positions[0].index).reverse().find((m) => !gone.has(m.id));
    const folderId = folderRef.current?.id;
    const unread = positions.filter((p) => !p.message.flags.seen).length;
    const openGone = selectedRef.current !== null && gone.has(selectedRef.current);

    setList((s) => (s.status === "ready" ? { ...s, items: s.items.filter((m) => !gone.has(m.id)), total: Math.max(0, s.total - gone.size) } : s));
    setChecked((sel) => prune(sel, current.filter((m) => !gone.has(m.id)).map((m) => m.id)));
    if (folderId && !searchingRef.current) setFolders((all) => all && adjustUnread(all, folderId, -unread));
    if (openGone) select((after ?? before)?.id ?? null);

    const restore = (only?: ReadonlySet<string>) => {
      if (folderRef.current?.id !== folderId) return;
      const back = only ? positions.filter((p) => only.has(p.message.id)) : positions;
      setList((s) => {
        if (s.status !== "ready") return s;
        const restored = [...s.items];
        let added = 0;
        for (const { message, index } of back) {
          if (restored.some((m) => m.id === message.id)) continue;
          restored.splice(Math.min(index, restored.length), 0, message);
          added++;
        }
        return { ...s, items: restored, total: s.total + added };
      });
      const backUnread = back.filter((p) => !p.message.flags.seen).length;
      if (folderId && !searchingRef.current) setFolders((all) => all && adjustUnread(all, folderId, backUnread));
    };

    const changed = () => {
      refreshFlaggedRef.current();
      void loadFoldersRef.current();
    };
    run()
      .then((result) => {
        changed();
        if (result.failed.length > 0) {
          restore(new Set(result.failed));
          toastRef.current({ kind: "error", title: failedTitle(result.failed.length), description: "Elas voltaram para a lista. Tente de novo em instantes." });
        }
        const batch = result.batch;
        if (!batch) return;
        toastRef.current({
          kind: "info",
          title,
          action: {
            label: "Desfazer",
            onClick: () => {
              api.undoMove(messages[0].accountId ?? account.id, batch)
                .then((back) => {
                  restore(new Set(back));
                  changed();
                  if (back.length < result.ids.length) {
                    toastRef.current({ kind: "warning", title: "Algumas conversas já tinham sido movidas", description: "Elas continuam no destino." });
                  }
                })
                .catch(() => toastRef.current({ kind: "error", title: "Não foi possível desfazer" }));
            },
          },
        });
      })
      .catch((error) => {
        restore();
        if (isAccessLost(error)) accessLostRef.current();
        toastRef.current({ kind: "error", title: failedTitle(messages.length), description: isAppError(error) ? error.message : "Elas voltaram para a lista." });
      });
  }, [account.id, select]);
  const loadFoldersRef = useRef(loadFolders);
  loadFoldersRef.current = loadFolders;
  const refreshFlaggedRef = useRef(refreshFlagged);
  refreshFlaggedRef.current = refreshFlagged;

  const onDelete = useCallback(() => {
    if (targets.length === 0) return;
    // Na Lixeira, excluir apaga de vez: pede confirmação antes, sem "Desfazer".
    if (targets.some((m) => roleOf(m) === "trash")) return setConfirmPurge(targets);
    removeWithUndo({
      messages: targets,
      title: batchTitle(targets.length, "Mensagem movida para a Lixeira", (n) => `${n} movidas para a Lixeira`),
      run: () => api.deleteMessages(targets.map((m) => m.id)),
      failedTitle: (n) => `${conversations(n)} não puderam ser excluídas`,
    });
  }, [removeWithUndo, roleOf, targets]);

  const purge = useCallback(async (messages: MessageHeader[]) => {
    setConfirmPurge(null);
    try {
      const result = await api.deleteMessages(messages.map((m) => m.id));
      const gone = new Set(result.ids);
      setList((s) => (s.status === "ready" ? { ...s, items: s.items.filter((m) => !gone.has(m.id)), total: Math.max(0, s.total - gone.size) } : s));
      setChecked((sel) => prune(sel, itemsRef.current.filter((m) => !gone.has(m.id)).map((m) => m.id)));
      if (selectedRef.current && gone.has(selectedRef.current)) select(null);
      if (result.failed.length > 0) toast({ kind: "error", title: `${conversations(result.failed.length)} não puderam ser excluídas` });
      else toast({ kind: "success", title: batchTitle(gone.size, "Mensagem excluída", (n) => `${n} excluídas`) });
    } catch (error) {
      fail(error, () => toast({ kind: "error", title: "Não foi possível excluir", description: isAppError(error) ? error.message : undefined }));
    }
  }, [fail, select, toast]);

  const onArchive = useCallback(() => {
    const messages = targets.filter((m) => roleOf(m) !== "all");
    if (messages.length === 0) return;
    removeWithUndo({
      messages,
      title: batchTitle(messages.length, "Mensagem arquivada", (n) => `${n} arquivadas`),
      run: () => api.archiveMessages(messages.map((m) => m.id)),
      failedTitle: (n) => `${conversations(n)} não puderam ser arquivadas`,
    });
  }, [removeWithUndo, roleOf, targets]);

  const moveTo = useCallback((messages: MessageHeader[], targetId: string, name: string) => {
    if (messages.length === 0) return;
    removeWithUndo({
      messages,
      title: batchTitle(messages.length, `Mensagem movida para ${name}`, (n) => `${n} movidas para ${name}`),
      run: () => api.moveMessages(messages.map((m) => m.id), targetId),
      failedTitle: (n) => `${conversations(n)} não puderam ser movidas`,
    });
  }, [removeWithUndo]);

  const onMove = useCallback((targetId: string) => {
    const owner = targets[0] ? targets[0].accountId ?? account.id : account.id;
    moveTo(targets, targetId, foldersOf(owner).find((f) => f.id === targetId)?.name ?? "a pasta");
  }, [account.id, foldersOf, moveTo, targets]);

  const onCreateAndMove = useCallback(async (name: string) => {
    if (targets.length === 0) return;
    const owner = targets[0].accountId ?? account.id;
    const created = await api.createFolder(owner, name);
    if (owner === account.id) await loadFolders();
    else setOtherFolders((map) => ({ ...map, [owner]: [...(map[owner] ?? []), created] }));
    moveTo(targets, created.id, created.name);
  }, [account.id, loadFolders, moveTo, targets]);

  // Arrastar mensagens até uma pasta (US5 CA3, por ponteiro na US6 CA5).
  const onDragMessages = useCallback((ids: string[] | null) => {
    setDragging(ids);
    if (!ids) setDropHover(null);
  }, []);
  const onDropMessages = useCallback((folderId: string, ids: string[]) => {
    const target = allFolders.find((f) => f.id === folderId);
    if (!target || !canReceive(target, sourceFolderId)) return;
    const moving = new Set(ids);
    moveTo(itemsRef.current.filter((m) => moving.has(m.id)), target.id, target.name);
  }, [allFolders, moveTo, sourceFolderId]);

  // ------------------------------------------------------------ lida, sinalizar, responder

  const onToggleRead = useCallback(() => void changeFlags(targets, readActionFor(targets)), [changeFlags, targets]);

  const onFollowup = useCallback(async (change: FollowupAction) => {
    if (targets.length === 0) return;
    try {
      const results = await api.setFollowup(targets.map((m) => m.id), change);
      const byId = new Map(results.map((r) => [r.id, r]));
      setList((s) => {
        if (s.status !== "ready") return s;
        return { ...s, items: s.items.map((m) => { const r = byId.get(m.id); return r ? { ...m, flags: r.flags, followup: r.followup } : m; }) };
      });
      refreshFlagged();
    } catch (error) {
      const title = targets.length > 1 ? `Não foi possível sinalizar ${conversations(targets.length)}` : "Não foi possível sinalizar a mensagem";
      fail(error, () => toast({ kind: "error", title, description: isAppError(error) ? error.message : undefined }));
    }
  }, [fail, refreshFlagged, targets, toast]);

  const onReply = useCallback((kind: Exclude<DraftKind, "new">) => {
    if (targets.length !== 1) return;
    if (selectedRef.current !== targets[0].id) select(targets[0].id);
    setCompose({ kind, messageId: targets[0].id });
  }, [select, targets]);

  const onNew = useCallback(() => {
    setCompose({ kind: "new", messageId: null });
    readerOpenRef.current(true);
  }, []);

  // ------------------------------------------------------------ envio (US7, CA1–CA3)

  /** Envios agendados aqui, para reabrir o card se o usuário desfizer ou se o envio falhar. */
  const sends = useRef(new Map<string, { compose: ComposeState; selected: string | null }>());
  const composeRef = useRef(compose);
  composeRef.current = compose;
  const foldersRef = useRef(folders);
  foldersRef.current = folders;

  const reopen = useCallback((entry: { compose: ComposeState; selected: string | null }) => {
    if (entry.compose.kind !== "new" && entry.selected) select(entry.selected);
    setCompose(entry.compose);
    readerOpenRef.current(true);
  }, [select]);

  const onSent = useCallback((sendId: string) => {
    const entry = { compose: composeRef.current ?? { kind: "new" as const, messageId: null }, selected: selectedRef.current };
    sends.current.set(sendId, entry);
    setCompose(null);
    if (!selectedRef.current) readerOpenRef.current(false);
    toast({
      kind: "info",
      title: "Enviando em 6 s",
      action: {
        label: "Desfazer",
        onClick: () => {
          api.cancelSend(sendId)
            .then((cancelled) => {
              if (!cancelled) return toastRef.current({ kind: "info", title: "A mensagem já foi enviada" });
              sends.current.delete(sendId);
              reopen(entry);
            })
            .catch(() => undefined);
        },
      },
    });
  }, [reopen, toast]);

  useEffect(() => onBackendEvent("send-finished", ({ sendId, accountId, ok, error }) => {
    if (accountId !== account.id) return;
    const entry = sends.current.get(sendId);
    sends.current.delete(sendId);
    if (ok) {
      const sent = foldersRef.current?.find((f) => f.role === "sent");
      toastRef.current({
        kind: "success",
        title: "Mensagem enviada",
        action: sent ? { label: "Ver em Enviados", onClick: () => { setFlagBucket(null); setFolder(sent); } } : undefined,
      });
      return;
    }
    if (error && isAccessLost(error)) accessLostRef.current();
    toastRef.current({
      kind: "error",
      title: "A mensagem não foi enviada",
      description: error ? `${error.message} O texto ficou no rascunho.` : "O texto ficou no rascunho.",
      action: entry ? { label: "Abrir rascunho", onClick: () => reopen(entry) } : undefined,
    });
  }), [account.id, reopen]);

  const notice =
    account.status === "signed_out" ? (
      <SectionMessage title="Você saiu desta conta" actions={[{ label: "Entrar de novo", onClick: onReconnect }]}>
        Estas são as mensagens salvas neste computador. Entre de novo para receber as novas.
      </SectionMessage>
    ) : account.status === "needs_reauth" ? (
      <SectionMessage appearance="warning" title="Reconecte esta conta" actions={[{ label: "Reconectar", onClick: onReconnect }]}>
        O servidor de e-mail encerrou o acesso do EpistulaMail a {account.email}. As mensagens salvas continuam disponíveis.
      </SectionMessage>
    ) : searching && searchTruncated ? (
      <SectionMessage title="Mostrando os 200 resultados mais recentes">
        Acrescente uma pessoa, um período ou outro filtro para ver os outros.
      </SectionMessage>
    ) : null;

  const savedNow = saved.find((s) => JSON.stringify(s.query) === queryKey) ?? null;
  const runSaved = (s: SavedSearch) => {
    setFlagBucket(null);
    onSearchChange(fromQuery(s.query, allFolders.find((f) => f.id === s.query.folder)?.name));
  };
  const setScope = (allAccounts: boolean) =>
    onSearchChange({ ...search, allAccounts, chips: allAccounts ? search.chips.filter((c) => c.kind !== "folder") : search.chips });
  const saveSearch = async () => {
    if (!saving) return;
    setSaving({ ...saving, busy: true, error: null });
    try {
      await api.saveSearch(account.id, saving.name, searchQuery);
      setSaving(null);
      loadSaved();
      toast({ kind: "success", title: `Busca "${saving.name.trim()}" salva`, description: "Ela fica na coluna de pastas." });
    } catch {
      setSaving({ ...saving, busy: false, error: "Não foi possível salvar a busca." });
    }
  };
  const deleteSaved = async (s: SavedSearch) => {
    try {
      await api.deleteSavedSearch(s.id);
      loadSaved();
      toast({ kind: "info", title: `Busca "${s.name}" excluída` });
    } catch {
      toast({ kind: "error", title: "Não foi possível excluir a busca salva" });
    }
  };
  const searchBar = searching ? (
    <SearchResultsBar
      count={list.status === "ready" ? list.items.length : null}
      truncated={searchTruncated}
      allAccounts={search.allAccounts}
      oldestFirst={search.oldestFirst}
      saved={!!savedNow}
      onScope={setScope}
      onOrder={(oldestFirst) => onSearchChange({ ...search, oldestFirst })}
      onSave={() => setSaving({ name: searchLabel(search).slice(0, 60), busy: false, error: null })}
    />
  ) : null;
  const ruleTarget = targetAccount;
  const applyRuleBlocked = !ruleTarget
    ? "Marque resultados de uma conta só para aplicar uma regra"
    : ruleTarget.status === "local"
      ? "Arquivos de e-mail não têm regras"
      : ruleTarget.status !== "active"
        ? "Entre de novo nesta conta para aplicar regras"
        : null;

  const listName = searching
    ? `Resultados para “${searchLabel(search)}”`
    : flagBucket
      ? `Sinalizadas: ${bucketLabel(flagBucket)}`
      : folder?.name ?? "Mensagens";
  const purgeInTrash = confirmPurge ? confirmPurge.filter((m) => roleOf(m) === "trash").length : 0;
  // Arquivo local (US15): não há servidor, a mensagem sai deste computador.
  const purgeFrom = account.status === "local" ? "deste computador" : "do servidor";

  return (
    <>
      {renderHeader(
        <CommandBar
          account={targetAccount ?? account}
          folders={foldersOf((targetAccount ?? account).id)}
          sourceFolderId={sourceFolderId}
          targets={targets}
          roleOf={roleOf}
          readOnly={!targetAccount || (targetAccount.status !== "active" && targetAccount.status !== "local")}
          local={(targetAccount ?? account).status === "local"}
          inert={settingsOpen}
          onNew={onNew}
          onDelete={onDelete}
          onArchive={onArchive}
          onMove={onMove}
          onCreateAndMove={onCreateAndMove}
          onReply={onReply}
          onToggleRead={onToggleRead}
          onFollowup={(change) => void onFollowup(change)}
          onRule={(intent) => void startRule(intent)}
          onApplyRule={searching ? () => ruleTarget && setApplyRule({ accountId: ruleTarget.id, ids: targets.map((m) => m.id) }) : undefined}
          applyRuleBlocked={applyRuleBlocked}
        />,
      )}
      {quickRule && (
        <QuickRuleDialog
          suggestions={quickRule.suggestions}
          folders={allFolders}
          intent={quickRule.intent}
          onCreate={async (name, spec, applyExisting) => {
            const saved = await api.saveRule(account.id, null, name, true, spec);
            let applied = 0;
            if (applyExisting) applied = await api.runRuleNow(account.id, saved.id, quickRule.suggestions.folderId).catch(() => 0);
            toast({ kind: "success", title: `Regra "${saved.name}" criada`, description: applyExisting ? `Aplicada a ${emailsLabel(applied)} que já estavam aqui.` : undefined });
          }}
          onMore={(name, spec) => {
            setQuickRule(null);
            onOpenRules({ accountId: account.id, draft: { name, spec } });
          }}
          onClose={() => setQuickRule(null)}
        />
      )}
      {applyRule && (
        <ApplyRuleDialog
          accountId={applyRule.accountId}
          messageIds={applyRule.ids}
          folders={foldersOf(applyRule.accountId)}
          onApplied={(count, rule) => {
            toast({
              kind: "success",
              title: `Regra "${rule.name}" aplicada a ${emailsLabel(count)}`,
              action: { label: "Ver histórico", onClick: () => onOpenRules({ accountId: applyRule.accountId, history: true }) },
            });
            setSearchReload((n) => n + 1);
            refreshFlagged();
            void loadFolders();
          }}
          onCreate={() => {
            const spec = ruleFromSearch(search);
            setApplyRule(null);
            onOpenRules({ accountId: applyRule.accountId, draft: { name: spec.conditions.length > 0 ? suggestedName(spec) : "", spec } });
          }}
          onClose={() => setApplyRule(null)}
        />
      )}
      {saving && (
        <Modal
          title="Salvar busca"
          initialFocus="#saved-search-name"
          onClose={() => setSaving(null)}
          footer={
            <>
              <Button appearance="subtle" onClick={() => setSaving(null)}>Cancelar</Button>
              <Button appearance="primary" loading={saving.busy} disabled={!saving.name.trim()} onClick={() => void saveSearch()}>Salvar</Button>
            </>
          }
        >
          <form
            className="field"
            onSubmit={(e) => {
              e.preventDefault();
              if (saving.name.trim()) void saveSearch();
            }}
          >
            <label className="field__label" htmlFor="saved-search-name">Nome</label>
            <div className={["input", saving.error && "is-invalid"].filter(Boolean).join(" ")}>
              <input id="saved-search-name" maxLength={60} value={saving.name} onChange={(e) => setSaving({ ...saving, name: e.target.value, error: null })} />
            </div>
            <span className={saving.error ? "field__help field__help--error" : "field__help"} role={saving.error ? "alert" : undefined}>
              {saving.error ?? (search.allAccounts ? "Aparece na coluna de pastas de todas as contas." : `Aparece na coluna de pastas de ${account.displayName}.`)}
            </span>
          </form>
        </Modal>
      )}
      <NavigationBlock>
        <FoldersColumn
          folders={folders}
          currentId={crossFolder ? null : folder?.id ?? null}
          flagged={visibleBuckets(flagCounts)}
          currentFlag={searching ? null : flagBucket}
          drag={dragging ? { sourceId: sourceFolderId, hoveredId: dropHover } : null}
          onSelect={(f) => {
            setFlagBucket(null);
            if (searching) onSearchChange({ chips: [], text: "", allAccounts: false, oldestFirst: false });
            setFolder(f);
          }}
          onSelectFlag={(bucket) => {
            setFlagReload(0);
            setFlagBucket(bucket);
          }}
          local={account.status === "local"}
          importing={importing}
          saved={saved}
          currentSavedId={searching ? savedNow?.id ?? null : null}
          onSelectSaved={runSaved}
          onDeleteSaved={(s) => void deleteSaved(s)}
        />
        <EmailListColumn
          folderName={listName}
          folderRole={folder?.role ?? "inbox"}
          searching={crossFolder}
          state={list}
          density={density}
          pager={pager}
          openId={selectedId}
          checked={checked.ids}
          master={masterState(checked, order)}
          syncing={syncing}
          canRefresh={account.status === "active"}
          notice={notice}
          searchBar={searchBar}
          emptySearch={
            <EmptyState
              title={search.allAccounts ? "Nenhum resultado" : "Nenhum resultado nesta conta"}
              description="Tente outras palavras, tire um filtro ou busque pelo nome ou e-mail de quem enviou."
              actions={search.allAccounts ? undefined : <Button onClick={() => setScope(true)}>Buscar em todas as contas</Button>}
            />
          }
          renderAccount={search.allAccounts ? (m) => <ResultAccount account={accountOf(m)} /> : undefined}
          renderBadge={(m) => <DueBadge followup={m.followup} />}
          onOpen={select}
          onCheck={onCheck}
          onCheckAll={onCheckAll}
          onClearChecked={clearChecked}
          canDrag={account.status === "active" || account.status === "local"}
          onDragMessages={onDragMessages}
          onDragOver={setDropHover}
          onDropMessages={onDropMessages}
          onRefresh={() => void refresh()}
          onNextPage={() => folder && void loadPage(folder, goForward(pager))}
          onPreviousPage={() => folder && void loadPage(folder, goBack(pager))}
          onToggleStar={(m) => void changeFlags([m], { flagged: !m.flags.flagged })}
          onToggleRead={(m) => void changeFlags([m], { seen: !m.flags.seen })}
        />
      </NavigationBlock>
      <ContentColumn>
        <MessageView
          account={accountOf(selected)}
          messageId={selectedId}
          badge={selected ? <DueBadge followup={selected.followup} /> : null}
          compose={compose}
          onCloseCompose={() => {
            setCompose(null);
            if (!selectedId) readerOpenRef.current(false);
          }}
          onSent={onSent}
          onLoaded={onLoaded}
          onAccessLost={accessLost}
          onBack={() => select(null)}
          onManageKeys={onManageKeys}
        />
      </ContentColumn>
      {confirmPurge && (
        <Modal
          title="Excluir para sempre?"
          icon="warning"
          initialFocus="[data-cancel]"
          onClose={() => setConfirmPurge(null)}
          footer={
            <>
              <Button appearance="subtle" data-cancel onClick={() => setConfirmPurge(null)}>Cancelar</Button>
              <Button appearance="danger" onClick={() => void purge(confirmPurge)}>Excluir para sempre</Button>
            </>
          }
        >
          <p>
            {purgeInTrash === 1
              ? `A mensagem que está na Lixeira será apagada ${purgeFrom} e não poderá ser recuperada.`
              : `As ${conversations(purgeInTrash)} que estão na Lixeira serão apagadas ${purgeFrom} e não poderão ser recuperadas.`}
            {confirmPurge.length > purgeInTrash && ` As outras ${conversations(confirmPurge.length - purgeInTrash)} vão para a Lixeira.`}
          </p>
        </Modal>
      )}
    </>
  );
}

/** Conta de um resultado na busca em todas as contas (L75): avatar mínimo antes da pasta. */
function ResultAccount({ account }: { account: Account }) {
  const local = account.status === "local";
  return (
    <span className="thread__account" role="img" aria-label={local ? `Arquivo ${account.displayName}` : `Conta ${account.displayName}`} data-tip={account.displayName}>
      <AccountAvatar initial={accountInitial(account.displayName)} colorClass={avatarColorClass(account.color)} photo={account.photo} size="xs" archive={local} />
    </span>
  );
}

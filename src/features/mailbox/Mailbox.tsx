// Composição de layout: Header (com a CommandBar), NavigationBlock (FoldersColumn + EmailListColumn) e ContentColumn.
// DS: toasts.html (desfazer e erros), mensagens.html (.section-message "Você saiu desta conta" / "Reconecte esta conta" /
// busca local), modais.html (confirmar exclusão definitiva na Lixeira)
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
} from "../../lib/tauri";
import { CommandBar } from "../../layout/CommandBar";
import { ContentColumn } from "../../layout/ContentColumn";
import { EmailListColumn, type ListState } from "../../layout/EmailListColumn";
import { FoldersColumn } from "../../layout/FoldersColumn";
import { NavigationBlock } from "../../layout/NavigationBlock";
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
  /** Busca do header (US4, CA1); a partir de 2 caracteres, a lista mostra os resultados. */
  query: string;
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

export function Mailbox({ account, initialFolderId, query, settingsOpen, renderHeader, density, onFolderChange, onAccessLost, onReconnect, onReaderOpenChange, appRequest }: MailboxProps) {
  const [folders, setFolders] = useState<Folder[] | null>(null);
  const [folder, setFolder] = useState<Folder | null>(null);
  const [pager, setPager] = useState<Pager>(firstPage);
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
  const searching = query.trim().length >= 2;
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

  // Busca local (US4, CA1): 250 ms depois da última tecla.
  useEffect(() => {
    if (!searching) return;
    const token = ++request.current;
    setList({ status: "loading" });
    setChecked(emptySelection);
    select(null);
    const timer = setTimeout(() => {
      api.searchMessages(account.id, query.trim())
        .then((page) => {
          if (token !== request.current) return;
          setSearchTruncated(page.truncated);
          setList({ status: "ready", items: page.items, total: page.items.length });
        })
        .catch((error) => {
          if (token !== request.current) return;
          setList({ status: "error", error: isAppError(error) ? error : FALLBACK });
        });
    }, SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  }, [account.id, query, searching, select]);

  // A sincronização em segundo plano avisa quando a pasta ou a lista de pastas mudou.
  useEffect(() => {
    const offs = [
      onBackendEvent("mailbox-updated", ({ accountId, folderId }) => {
        if (accountId === account.id) loadFlagCounts();
        const current = folderRef.current;
        if (accountId !== account.id || !current || current.id !== folderId) return;
        void loadPage(current, pagerRef.current, true);
      }),
      onBackendEvent("folders-updated", ({ accountId }) => {
        if (accountId === account.id) void loadFolders();
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
  }, [account.id, loadFlagCounts, loadFolders, loadPage]);

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
  const allFolders = useMemo(() => folders ?? [], [folders]);
  const sourceFolderId = crossFolder ? null : folder?.id ?? null;
  const roleOf = useCallback(
    (m: MessageHeader) => allFolders.find((f) => f.id === (crossFolder ? m.folderId : folder?.id))?.role,
    [allFolders, folder?.id, crossFolder],
  );

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
              api.undoMove(account.id, batch)
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
    moveTo(targets, targetId, allFolders.find((f) => f.id === targetId)?.name ?? "a pasta");
  }, [allFolders, moveTo, targets]);

  const onCreateAndMove = useCallback(async (name: string) => {
    if (targets.length === 0) return;
    const created = await api.createFolder(account.id, name);
    await loadFolders();
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
        O servidor de e-mail encerrou o acesso do Epistula a {account.email}. As mensagens salvas continuam disponíveis.
      </SectionMessage>
    ) : searching ? (
      <SectionMessage title="Buscando nas mensagens salvas neste computador">
        {searchTruncated ? "Mostrando os 200 resultados mais recentes. Refine a busca para ver outros." : "Mensagens que ainda não foram baixadas não aparecem aqui."}
      </SectionMessage>
    ) : null;

  const listName = searching
    ? `Resultados para “${query.trim()}”`
    : flagBucket
      ? `Sinalizadas: ${bucketLabel(flagBucket)}`
      : folder?.name ?? "Mensagens";
  const purgeInTrash = confirmPurge ? confirmPurge.filter((m) => roleOf(m) === "trash").length : 0;

  return (
    <>
      {renderHeader(
        <CommandBar
          account={account}
          folders={allFolders}
          sourceFolderId={sourceFolderId}
          targets={targets}
          roleOf={roleOf}
          readOnly={account.status !== "active"}
          inert={settingsOpen}
          onNew={onNew}
          onDelete={onDelete}
          onArchive={onArchive}
          onMove={onMove}
          onCreateAndMove={onCreateAndMove}
          onReply={onReply}
          onToggleRead={onToggleRead}
          onFollowup={(change) => void onFollowup(change)}
        />,
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
            setFolder(f);
          }}
          onSelectFlag={(bucket) => {
            setFlagReload(0);
            setFlagBucket(bucket);
          }}
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
          renderBadge={(m) => <DueBadge followup={m.followup} />}
          onOpen={select}
          onCheck={onCheck}
          onCheckAll={onCheckAll}
          onClearChecked={clearChecked}
          canDrag={account.status === "active"}
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
          account={account}
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
              ? "A mensagem que está na Lixeira será apagada do servidor e não poderá ser recuperada."
              : `As ${conversations(purgeInTrash)} que estão na Lixeira serão apagadas do servidor e não poderão ser recuperadas.`}
            {confirmPurge.length > purgeInTrash && ` As outras ${conversations(confirmPurge.length - purgeInTrash)} vão para a Lixeira.`}
          </p>
        </Modal>
      )}
    </>
  );
}

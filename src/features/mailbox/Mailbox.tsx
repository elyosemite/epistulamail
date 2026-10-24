// Composição de DS: espacamento.html (.app-shell__commands, __sidebar, __list, __reader) + toasts.html (desfazer e erros)
// + mensagens.html (.section-message "Você saiu desta conta" / "Reconecte esta conta" / busca local)
// + modais.html (confirmar exclusão definitiva na Lixeira)
import { useCallback, useEffect, useRef, useState } from "react";
import { dueLabel } from "../../lib/followup";
import { adjustUnread, applyFlags, restoreFlags, snapshotFlags, unreadDelta } from "../../lib/flags";
import { currentCursor, firstPage, goBack, goForward, withNext, type Pager } from "../../lib/pagination";
import {
  api,
  isAccessLost,
  isAppError,
  onBackendEvent,
  type Account,
  type DraftKind,
  type Folder,
  type FollowupAction,
  type Followup,
  type MessageFlags,
  type MessageHeader,
} from "../../lib/tauri";
import { UndoQueue } from "../../lib/undo";
import { Button } from "../../ui/Button";
import { Lozenge } from "../../ui/Lozenge";
import { Modal } from "../../ui/Modal";
import { SectionMessage } from "../../ui/SectionMessage";
import { useToast } from "../../ui/Toast";
import { InboxList, type ListState } from "../inbox/InboxList";
import { MessageView, type ComposeState } from "../message/MessageView";
import { FolderNav } from "./FolderNav";
import { MailCommands } from "./MailCommands";

interface MailboxProps {
  account: Account;
  initialFolderId: string | null;
  /** Busca do header (US4, CA1); a partir de 2 caracteres, a lista mostra os resultados. */
  query: string;
  onFolderChange: (folderId: string) => void;
  /** O backend recusou por falta de acesso: o App recarrega as contas (status mudou). */
  onAccessLost: () => void;
  onReconnect: () => void;
  onReaderOpenChange: (open: boolean) => void;
}

const FALLBACK = { kind: "ImapConnection" as const, message: "Tente de novo em instantes." };
const SEARCH_DELAY_MS = 250;

const TONE = { today: "moved", future: "default", overdue: "overdue", done: "success" } as const;

/** Lozenge do prazo da sinalização ("Hoje", "Amanhã", "Atrasada"…). */
export function DueBadge({ followup }: { followup: Followup | null }) {
  const label = dueLabel(followup);
  if (!label || label.tone === "none") return null;
  return <Lozenge appearance={TONE[label.tone]}>{label.text}</Lozenge>;
}

export function Mailbox({ account, initialFolderId, query, onFolderChange, onAccessLost, onReconnect, onReaderOpenChange }: MailboxProps) {
  const [folders, setFolders] = useState<Folder[] | null>(null);
  const [folder, setFolder] = useState<Folder | null>(null);
  const [pager, setPager] = useState<Pager>(firstPage);
  const [list, setList] = useState<ListState>({ status: "loading" });
  const [syncing, setSyncing] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [compose, setCompose] = useState<ComposeState | null>(null);
  const [confirmPurge, setConfirmPurge] = useState<MessageHeader | null>(null);
  const [searchTruncated, setSearchTruncated] = useState(false);
  const toast = useToast();
  const request = useRef(0);
  // Leitura das mensagens atuais sem fazer callbacks estáveis dependerem da lista inteira.
  const itemsRef = useRef<MessageHeader[]>([]);
  itemsRef.current = list.status === "ready" ? list.items : [];
  const pagerRef = useRef(pager);
  pagerRef.current = pager;
  const folderRef = useRef(folder);
  folderRef.current = folder;
  const initialRef = useRef(initialFolderId);
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
  const searchingRef = useRef(searching);
  searchingRef.current = searching;

  const fail = useCallback((error: unknown, onOther: () => void) => {
    if (isAccessLost(error)) accessLostRef.current();
    if (isAccessLost(error) && error.kind === "SignedOut") return;
    onOther();
  }, []);

  const undo = useRef<UndoQueue | null>(null);
  if (!undo.current) {
    undo.current = new UndoQueue(undefined, (_action, error) => {
      if (isAccessLost(error)) accessLostRef.current();
      toastRef.current({ kind: "error", title: "Não foi possível concluir a ação", description: isAppError(error) ? error.message : "A mensagem voltou para a lista." });
    });
  }
  const toastRef = useRef(toast);
  toastRef.current = toast;
  // Troca de conta ou fechamento: o que estava esperando o "Desfazer" vai para o servidor agora.
  useEffect(() => () => undo.current?.flush(), []);

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
    setCompose((c) => (c?.kind === "new" ? c : null));
    readerOpenRef.current(id !== null);
  }, []);

  /** `silent`: recarrega sem esqueleto (atualização vinda da sincronização em segundo plano). */
  const loadPage = useCallback(async (target: Folder, page: Pager, silent = false) => {
    if (searchingRef.current) return;
    const token = ++request.current;
    if (!silent) setList({ status: "loading" });
    try {
      const result = await api.listMessages(target.id, currentCursor(page));
      if (token !== request.current) return;
      setPager(withNext(page, result.nextCursor));
      setList({ status: "ready", items: result.items, total: result.total });
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
    if (!folder || searching) return;
    const page = firstPage();
    setPager(page);
    select(null);
    void loadPage(folder, page);
    folderChangeRef.current(folder.id);
    // Só ao trocar de pasta ou ao sair da busca: o objeto `folder` muda também com os contadores.
  }, [folder?.id, searching, loadPage, select]);

  // Busca local (CA1): 250 ms depois da última tecla.
  useEffect(() => {
    if (!searching) return;
    const token = ++request.current;
    setList({ status: "loading" });
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
        const current = folderRef.current;
        if (accountId !== account.id || !current || current.id !== folderId) return;
        void loadPage(current, pagerRef.current, true);
      }),
      onBackendEvent("folders-updated", ({ accountId }) => {
        if (accountId === account.id) void loadFolders();
      }),
    ];
    return () => offs.forEach((off) => off());
  }, [account.id, loadFolders, loadPage]);

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
    } catch (error) {
      setList((s) => (s.status === "ready" ? { ...s, items: restoreFlags(s.items, previous) } : s));
      if (folderId && !searchingRef.current) setFolders((all) => all && adjustUnread(all, folderId, -delta));
      if (isAccessLost(error)) accessLost();
      toast({
        kind: "error",
        title: "Não foi possível atualizar a mensagem",
        description: isAppError(error) ? error.message : undefined,
        action: isAccessLost(error) ? undefined : { label: "Tentar de novo", onClick: () => void changeFlags(messages, change) },
      });
    }
  }, [accessLost, toast]);

  // Abrir marca como lida no servidor; a lista e o contador acompanham sem nova chamada.
  const onLoaded = useCallback((id: string, flags: MessageFlags) => {
    const before = itemsRef.current.find((m) => m.id === id);
    const current = folderRef.current;
    if (before && !before.flags.seen && flags.seen && current && !searchingRef.current) {
      setFolders((all) => all && adjustUnread(all, current.id, -1));
    }
    setList((s) => (s.status === "ready" ? { ...s, items: s.items.map((m) => (m.id === id ? { ...m, flags } : m)) } : s));
  }, []);

  // ------------------------------------------------------------ excluir, arquivar, mover (CA4, CA5)

  /** Tira a mensagem da lista na hora, seleciona a próxima e agenda a chamada com "Desfazer". */
  const removeWithUndo = useCallback((message: MessageHeader, title: string, run: () => Promise<unknown>) => {
    const items = itemsRef.current;
    const index = items.findIndex((m) => m.id === message.id);
    if (index < 0) return;
    const next = items[index + 1] ?? items[index - 1] ?? null;
    const folderId = folderRef.current?.id;
    const unread = message.flags.seen ? 0 : 1;
    setList((s) => (s.status === "ready" ? { ...s, items: s.items.filter((m) => m.id !== message.id), total: Math.max(0, s.total - 1) } : s));
    if (folderId && !searchingRef.current) setFolders((all) => all && adjustUnread(all, folderId, -unread));
    select(next?.id ?? null);
    undo.current!.schedule({
      run: async () => {
        await run();
      },
      restore: () => {
        if (folderRef.current?.id !== folderId) return;
        setList((s) => {
          if (s.status !== "ready" || s.items.some((m) => m.id === message.id)) return s;
          const items = [...s.items];
          items.splice(Math.min(index, items.length), 0, message);
          return { ...s, items, total: s.total + 1 };
        });
        if (folderId && !searchingRef.current) setFolders((all) => all && adjustUnread(all, folderId, unread));
      },
    });
    toast({ kind: "info", title, action: { label: "Desfazer", onClick: () => undo.current!.undo() } });
  }, [select, toast]);

  const selected = list.status === "ready" ? list.items.find((m) => m.id === selectedId) ?? null : null;
  const allFolders = folders ?? [];
  const sourceFolderId = selected ? (searching ? selected.folderId ?? null : folder?.id ?? null) : null;
  const sourceFolder = allFolders.find((f) => f.id === sourceFolderId);

  const onDelete = useCallback(() => {
    if (!selected) return;
    if (sourceFolder?.role === "trash") return setConfirmPurge(selected);
    removeWithUndo(selected, "Mensagem movida para a Lixeira", () => api.deleteMessages([selected.id]));
  }, [removeWithUndo, selected, sourceFolder?.role]);

  const purge = useCallback(async (message: MessageHeader) => {
    setConfirmPurge(null);
    try {
      await api.deleteMessages([message.id]);
      setList((s) => (s.status === "ready" ? { ...s, items: s.items.filter((m) => m.id !== message.id), total: Math.max(0, s.total - 1) } : s));
      select(null);
      toast({ kind: "success", title: "Mensagem excluída para sempre" });
    } catch (error) {
      fail(error, () => toast({ kind: "error", title: "Não foi possível excluir", description: isAppError(error) ? error.message : undefined }));
    }
  }, [fail, select, toast]);

  const onArchive = useCallback(() => {
    if (!selected) return;
    removeWithUndo(selected, "Mensagem arquivada", () => api.archiveMessages([selected.id]));
  }, [removeWithUndo, selected]);

  const onMove = useCallback((targetId: string) => {
    if (!selected) return;
    const name = allFolders.find((f) => f.id === targetId)?.name ?? "a pasta";
    removeWithUndo(selected, `Mensagem movida para ${name}`, () => api.moveMessages([selected.id], targetId));
  }, [allFolders, removeWithUndo, selected]);

  const onCreateAndMove = useCallback(async (name: string) => {
    if (!selected) return;
    const created = await api.createFolder(account.id, name);
    await loadFolders();
    removeWithUndo(selected, `Pasta "${created.name}" criada; mensagem movida`, () => api.moveMessages([selected.id], created.id));
  }, [account.id, loadFolders, removeWithUndo, selected]);

  // ------------------------------------------------------------ sinalizar (CA7)

  const onFollowup = useCallback(async (change: FollowupAction) => {
    if (!selected) return;
    const id = selected.id;
    try {
      const result = await api.setFollowup(id, change);
      setList((s) => (s.status === "ready" ? { ...s, items: s.items.map((m) => (m.id === id ? { ...m, flags: result.flags, followup: result.followup } : m)) } : s));
    } catch (error) {
      fail(error, () => toast({ kind: "error", title: "Não foi possível sinalizar a mensagem", description: isAppError(error) ? error.message : undefined }));
    }
  }, [fail, selected, toast]);

  const onReply = useCallback((kind: Exclude<DraftKind, "new">) => {
    if (selected) setCompose({ kind, messageId: selected.id });
  }, [selected]);

  const onNew = useCallback(() => {
    setCompose({ kind: "new", messageId: null });
    readerOpenRef.current(true);
  }, []);

  const notice =
    account.status === "signed_out" ? (
      <SectionMessage title="Você saiu desta conta" actions={[{ label: "Entrar de novo", onClick: onReconnect }]}>
        Estas são as mensagens salvas neste computador. Entre de novo para receber as novas.
      </SectionMessage>
    ) : account.status === "needs_reauth" ? (
      <SectionMessage appearance="warning" title="Reconecte esta conta" actions={[{ label: "Reconectar", onClick: onReconnect }]}>
        O Google encerrou o acesso do Epistula a {account.email}. As mensagens salvas continuam disponíveis.
      </SectionMessage>
    ) : searching ? (
      <SectionMessage title="Buscando nas mensagens salvas neste computador">
        {searchTruncated ? "Mostrando os 200 resultados mais recentes. Refine a busca para ver outros." : "Mensagens que ainda não foram baixadas não aparecem aqui."}
      </SectionMessage>
    ) : null;

  return (
    <>
      <MailCommands
        account={account}
        folders={allFolders}
        sourceFolderId={sourceFolderId}
        selected={selected}
        readOnly={account.status !== "active"}
        onNew={onNew}
        onDelete={onDelete}
        onArchive={onArchive}
        onMove={onMove}
        onCreateAndMove={onCreateAndMove}
        onReply={onReply}
        onToggleRead={() => selected && void changeFlags([selected], { seen: !selected.flags.seen })}
        onFollowup={(change) => void onFollowup(change)}
      />
      <FolderNav folders={folders} currentId={searching ? null : folder?.id ?? null} onSelect={(f) => setFolder(f)} />
      <section className="app-shell__list" aria-label={searching ? "Resultados da busca" : folder?.name ?? "Mensagens"}>
        <InboxList
          folderName={searching ? `Resultados para “${query.trim()}”` : folder?.name ?? "Mensagens"}
          folderRole={folder?.role ?? "inbox"}
          searching={searching}
          state={list}
          pager={pager}
          selectedId={selectedId}
          syncing={syncing}
          canRefresh={account.status === "active"}
          notice={notice}
          renderBadge={(m) => <DueBadge followup={m.followup} />}
          onSelect={select}
          onRefresh={() => void refresh()}
          onNextPage={() => folder && void loadPage(folder, goForward(pager))}
          onPreviousPage={() => folder && void loadPage(folder, goBack(pager))}
          onToggleStar={(m) => void changeFlags([m], { flagged: !m.flags.flagged })}
          onToggleRead={(m) => void changeFlags([m], { seen: !m.flags.seen })}
        />
      </section>
      <section className="app-shell__reader" aria-label="Mensagem">
        <MessageView
          account={account}
          messageId={selectedId}
          badge={selected ? <DueBadge followup={selected.followup} /> : null}
          compose={compose}
          onCloseCompose={() => {
            setCompose(null);
            if (!selectedId) readerOpenRef.current(false);
          }}
          onLoaded={onLoaded}
          onAccessLost={accessLost}
          onBack={() => select(null)}
        />
      </section>
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
          <p>A mensagem será apagada do Gmail e não poderá ser recuperada.</p>
        </Modal>
      )}
    </>
  );
}

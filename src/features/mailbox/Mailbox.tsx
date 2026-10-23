// Composição de DS: espacamento.html (.app-shell__sidebar, __list, __reader) + toasts.html (erros de flag)
import { useCallback, useEffect, useRef, useState } from "react";
import { adjustUnread, applyFlags, restoreFlags, snapshotFlags, unreadDelta } from "../../lib/flags";
import { currentCursor, firstPage, goBack, goForward, withNext, type Pager } from "../../lib/pagination";
import { api, isAppError, isSessionLost, type AppErrorKind, type Folder, type MessageFlags, type MessageHeader } from "../../lib/tauri";
import { useToast } from "../../ui/Toast";
import { InboxList, type ListState } from "../inbox/InboxList";
import { MessageView } from "../message/MessageView";
import { FolderNav } from "./FolderNav";

interface MailboxProps {
  onSessionLost: (kind: AppErrorKind) => void;
  onReaderOpenChange: (open: boolean) => void;
}

const FALLBACK = { kind: "ImapConnection" as const, message: "Tente de novo em instantes." };

export function Mailbox({ onSessionLost, onReaderOpenChange }: MailboxProps) {
  const [folders, setFolders] = useState<Folder[] | null>(null);
  const [folder, setFolder] = useState<Folder | null>(null);
  const [pager, setPager] = useState<Pager>(firstPage);
  const [list, setList] = useState<ListState>({ status: "loading" });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const toast = useToast();
  const request = useRef(0);
  // Leitura das mensagens atuais sem fazer callbacks estáveis dependerem da lista inteira.
  const itemsRef = useRef<MessageHeader[]>([]);
  itemsRef.current = list.status === "ready" ? list.items : [];

  const fail = useCallback((error: unknown, onOther: () => void) => {
    if (isSessionLost(error)) onSessionLost(error.kind);
    else onOther();
  }, [onSessionLost]);

  useEffect(() => {
    api.listFolders()
      .then((all) => {
        setFolders(all);
        setFolder(all.find((f) => f.role === "inbox") ?? all[0] ?? null);
      })
      .catch((error) => fail(error, () => {
        setFolders([]);
        setList({ status: "error", error: isAppError(error) ? error : FALLBACK });
      }));
  }, [fail]);

  const select = useCallback((id: string | null) => {
    setSelectedId(id);
    onReaderOpenChange(id !== null);
  }, [onReaderOpenChange]);

  const loadPage = useCallback(async (target: Folder, page: Pager) => {
    const token = ++request.current;
    setList({ status: "loading" });
    try {
      const result = await api.listMessages(target.id, currentCursor(page));
      if (token !== request.current) return;
      setPager(withNext(page, result.nextCursor));
      setList({ status: "ready", items: result.items, total: result.total });
      if (result.unread !== null) {
        setFolders((all) => all && all.map((f) => (f.id === target.id ? { ...f, unread: result.unread } : f)));
      }
    } catch (error) {
      if (token !== request.current) return;
      if (isAppError(error) && error.kind === "CursorExpired") return void loadPage(target, firstPage());
      fail(error, () => setList({ status: "error", error: isAppError(error) ? error : FALLBACK }));
    }
  }, [fail]);

  useEffect(() => {
    if (!folder) return;
    const page = firstPage();
    setPager(page);
    select(null);
    void loadPage(folder, page);
  }, [folder, loadPage, select]);

  const changeFlags = useCallback(async (messages: MessageHeader[], change: Partial<MessageFlags>) => {
    if (!folder || messages.length === 0) return;
    const ids = messages.map((m) => m.id);
    const previous = snapshotFlags(itemsRef.current, ids);
    if (previous.size === 0) return;
    const delta = unreadDelta(previous, change);
    setList((s) => (s.status === "ready" ? { ...s, items: applyFlags(s.items, ids, change) } : s));
    setFolders((all) => all && adjustUnread(all, folder.id, delta));
    try {
      const results = await api.setFlags(ids, change);
      const confirmed = new Map(results.map((r) => [r.id, r.flags]));
      setList((s) => (s.status === "ready" ? { ...s, items: s.items.map((m) => ({ ...m, flags: confirmed.get(m.id) ?? m.flags })) } : s));
    } catch (error) {
      setList((s) => (s.status === "ready" ? { ...s, items: restoreFlags(s.items, previous) } : s));
      setFolders((all) => all && adjustUnread(all, folder.id, -delta));
      fail(error, () =>
        toast({
          kind: "error",
          title: "Não foi possível atualizar a mensagem",
          description: isAppError(error) ? error.message : undefined,
          action: { label: "Tentar de novo", onClick: () => void changeFlags(messages, change) },
        }),
      );
    }
  }, [fail, folder, toast]);

  // Abrir marca como lida no servidor; a lista e o contador acompanham sem nova chamada.
  const onLoaded = useCallback((id: string, flags: MessageFlags) => {
    const before = itemsRef.current.find((m) => m.id === id);
    if (before && !before.flags.seen && flags.seen && folder) {
      setFolders((all) => all && adjustUnread(all, folder.id, -1));
    }
    setList((s) => (s.status === "ready" ? { ...s, items: s.items.map((m) => (m.id === id ? { ...m, flags } : m)) } : s));
  }, [folder]);

  const selected = list.status === "ready" ? list.items.find((m) => m.id === selectedId) ?? null : null;

  return (
    <>
      <FolderNav folders={folders} currentId={folder?.id ?? null} onSelect={(f) => setFolder(f)} />
      <section className="app-shell__list" aria-label={folder?.name ?? "Mensagens"}>
        <InboxList
          folderName={folder?.name ?? "Mensagens"}
          folderRole={folder?.role ?? "inbox"}
          state={list}
          pager={pager}
          selectedId={selectedId}
          onSelect={select}
          onRefresh={() => folder && void loadPage(folder, pager)}
          onNextPage={() => folder && void loadPage(folder, goForward(pager))}
          onPreviousPage={() => folder && void loadPage(folder, goBack(pager))}
          onToggleStar={(m) => void changeFlags([m], { flagged: !m.flags.flagged })}
          onToggleRead={(m) => void changeFlags([m], { seen: !m.flags.seen })}
        />
      </section>
      <section className="app-shell__reader" aria-label="Mensagem">
        <MessageView
          messageId={selectedId}
          flags={selected?.flags ?? null}
          onLoaded={onLoaded}
          onToggleStar={() => selected && void changeFlags([selected], { flagged: !selected.flags.flagged })}
          onMarkUnread={() => selected && void changeFlags([selected], { seen: false })}
          onSessionLost={onSessionLost}
          onBack={() => select(null)}
        />
      </section>
    </>
  );
}

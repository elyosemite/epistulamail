// Composição de DS: threads.html (.toolbar com paginação, .thread-list, .thread), carregamento.html (skeleton e
// .spinner--sm de sincronização), vazios.html (.empty)
import { useRef, type KeyboardEvent, type ReactNode } from "react";
import { formatListDate, senderLabel } from "../../lib/format";
import { canGoBack, rangeLabel, type Pager } from "../../lib/pagination";
import type { AppError, FolderRole, MessageHeader } from "../../lib/tauri";
import { Button, IconButton } from "../../ui/Button";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { ThreadList, ThreadRow, ThreadSkeleton } from "../../ui/Thread";
import { Toolbar } from "../../ui/Toolbar";

export type ListState =
  | { status: "loading" }
  | { status: "ready"; items: MessageHeader[]; total: number }
  | { status: "error"; error: AppError };

const SKELETON_ROWS: Array<[string, string]> = [["40%", "70%"], ["30%", "55%"], ["45%", "65%"], ["35%", "80%"], ["25%", "60%"], ["40%", "50%"]];

interface InboxListProps {
  folderName: string;
  folderRole: FolderRole;
  state: ListState;
  pager: Pager;
  selectedId: string | null;
  /** Sincronização da pasta em andamento (spinner na toolbar). */
  syncing: boolean;
  /** Conta sem acesso ao servidor (saiu ou precisa reconectar): "Atualizar" fica desabilitado. */
  canRefresh: boolean;
  /** Aviso acima da lista (section message da conta ou da busca). */
  notice?: ReactNode;
  /** Resultados de busca: sem paginação nem "Atualizar"; cada linha mostra a pasta. */
  searching?: boolean;
  /** Prazo da sinalização de cada linha. */
  renderBadge?: (message: MessageHeader) => ReactNode;
  onSelect: (id: string) => void;
  onRefresh: () => void;
  onNextPage: () => void;
  onPreviousPage: () => void;
  onToggleStar: (message: MessageHeader) => void;
  onToggleRead: (message: MessageHeader) => void;
}

export function InboxList(props: InboxListProps) {
  const { folderName, folderRole, state, pager, selectedId, syncing, canRefresh, notice, searching, renderBadge, onSelect, onRefresh, onNextPage, onPreviousPage, onToggleStar, onToggleRead } = props;
  const rows = useRef(new Map<string, HTMLDivElement>());
  const loading = state.status === "loading";

  function onRowKeyDown(event: KeyboardEvent<HTMLDivElement>, index: number) {
    if (state.status !== "ready") return;
    const { items } = state;
    const move = (to: number) => {
      const target = items[Math.max(0, Math.min(items.length - 1, to))];
      onSelect(target.id);
      rows.current.get(target.id)?.focus();
    };
    const message = items[index];
    if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(message.id); }
    else if (event.key === "j" || event.key === "ArrowDown") { event.preventDefault(); move(index + 1); }
    else if (event.key === "k" || event.key === "ArrowUp") { event.preventDefault(); move(index - 1); }
    else if (event.key === "s") { event.preventDefault(); onToggleStar(message); }
    else if (event.key === "U" && event.shiftKey) { event.preventDefault(); onToggleRead(message); }
  }

  const end = state.status === "ready" && (searching ? (
    <span>{state.items.length === 1 ? "1 resultado" : `${state.items.length.toLocaleString("pt-BR")} resultados`}</span>
  ) : (
    <>
      <span>{rangeLabel(pager, state.items.length, state.total)}</span>
      <IconButton icon="chevron-left" label="Página anterior" compact disabled={!canGoBack(pager)} onClick={onPreviousPage} />
      <IconButton icon="chevron-right" label="Próxima página" compact disabled={!pager.next} onClick={onNextPage} />
    </>
  ));

  return (
    <>
      <Toolbar label={`Ações de ${folderName}`} end={end}>
        <IconButton icon="refresh" label="Atualizar" onClick={onRefresh} disabled={searching || loading || syncing || !canRefresh} />
        {syncing && <Spinner size="sm" label="Sincronizando" />}
      </Toolbar>
      <div className="app-shell__list-body">
        {notice && <div style={{ padding: "var(--space-150) var(--space-200) 0" }}>{notice}</div>}
        {state.status === "loading" && (
          <ThreadList label="Carregando conversas" busy>
            {SKELETON_ROWS.map(([from, subject], i) => <ThreadSkeleton key={i} from={from} subject={subject} />)}
          </ThreadList>
        )}
        {state.status === "error" && (
          <EmptyState
            art="error"
            title="Não foi possível carregar a pasta"
            description={state.error.message}
            actions={<Button appearance="primary" onClick={onRefresh}>Tentar de novo</Button>}
          />
        )}
        {state.status === "ready" && state.items.length === 0 && (
          searching
            ? <EmptyState title="Nenhuma mensagem encontrada" description="Tente outras palavras, o nome ou o e-mail de quem enviou." />
            : folderRole === "inbox"
            ? <EmptyState art="done" title="Tudo em dia" description="Não há mensagens na Caixa de entrada." />
            : <EmptyState title="Nenhuma mensagem nesta pasta" description={`${folderName} está vazia.`} />
        )}
        {state.status === "ready" && state.items.length > 0 && (
          <ThreadList label={folderName}>
            {state.items.map((m, index) => (
              <ThreadRow
                key={m.id}
                ref={(el) => { if (el) rows.current.set(m.id, el); else rows.current.delete(m.id); }}
                from={senderLabel(m.fromName, m.fromAddress)}
                subject={m.subject || "(sem assunto)"}
                date={formatListDate(m.date)}
                unread={!m.flags.seen}
                starred={m.flags.flagged}
                open={m.id === selectedId}
                onOpen={() => onSelect(m.id)}
                onKeyDown={(e) => onRowKeyDown(e, index)}
                onToggleStar={() => onToggleStar(m)}
                onToggleRead={() => onToggleRead(m)}
                badge={renderBadge?.(m)}
                folder={searching ? m.folderName : undefined}
              />
            ))}
          </ThreadList>
        )}
      </div>
    </>
  );
}

// Composição de DS: threads.html (.toolbar com checkbox da página, .is-selecting, .toolbar__count e paginação),
// espacamento.html (.app-shell__list no bloco de navegação), carregamento.html (skeleton e .spinner--sm de
// sincronização), vazios.html (.empty)
import type { ReactNode } from "react";
import { canGoBack, rangeLabel, type Pager } from "../lib/pagination";
import type { AppError, Density, FolderRole, MessageHeader } from "../lib/tauri";
import { Button, IconButton } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { Spinner } from "../ui/Spinner";
import { ThreadList, ThreadSkeleton } from "../ui/Thread";
import { Toolbar } from "../ui/Toolbar";
import { EmailList, type DragHandlers } from "./EmailList";

export type ListState =
  | { status: "loading" }
  | { status: "ready"; items: MessageHeader[]; total: number }
  | { status: "error"; error: AppError };

const SKELETON_ROWS: Array<[string, string]> = [["40%", "70%"], ["30%", "55%"], ["45%", "65%"], ["35%", "80%"], ["25%", "60%"], ["40%", "50%"]];

interface EmailListColumnProps extends DragHandlers {
  folderName: string;
  folderRole: FolderRole;
  state: ListState;
  density: Density;
  pager: Pager;
  openId: string | null;
  checked: ReadonlySet<string>;
  /** Estado do checkbox da página. */
  master: "none" | "some" | "all";
  /** Sincronização da pasta em andamento (spinner na toolbar). */
  syncing: boolean;
  /** Conta sem acesso ao servidor (saiu ou precisa reconectar): "Atualizar" fica desabilitado. */
  canRefresh: boolean;
  /** Aviso acima da lista (section message da conta ou da busca). */
  notice?: ReactNode;
  /** Resultados de busca ou de um prazo das sinalizadas: sem paginação nem "Atualizar"; cada linha mostra a pasta. */
  searching?: boolean;
  /** Arrastar desligado (conta sem acesso ao servidor). */
  canDrag: boolean;
  /** Prazo da sinalização de cada linha. */
  renderBadge: (message: MessageHeader) => ReactNode;
  onOpen: (id: string) => void;
  onCheck: (id: string, range: boolean) => void;
  onCheckAll: () => void;
  onClearChecked: () => void;
  onRefresh: () => void;
  onNextPage: () => void;
  onPreviousPage: () => void;
  onToggleStar: (message: MessageHeader) => void;
  onToggleRead: (message: MessageHeader) => void;
}

/** Coluna da lista de e-mails: toolbar (seleção, atualizar, paginação) e a lista. */
export function EmailListColumn(props: EmailListColumnProps) {
  const { folderName, folderRole, state, pager, openId, checked, master, syncing, canRefresh, notice, searching, renderBadge } = props;
  const loading = state.status === "loading";
  const selecting = checked.size > 0;

  const end = state.status === "ready" && (searching ? (
    <span>{state.items.length === 1 ? "1 resultado" : `${state.items.length.toLocaleString("pt-BR")} resultados`}</span>
  ) : (
    <>
      <span>{rangeLabel(pager, state.items.length, state.total)}</span>
      <IconButton icon="chevron-left" label="Página anterior" compact disabled={!canGoBack(pager)} onClick={props.onPreviousPage} />
      <IconButton icon="chevron-right" label="Próxima página" compact disabled={!pager.next} onClick={props.onNextPage} />
    </>
  ));

  return (
    <section className="app-shell__list" aria-label={folderName}>
      <Toolbar label={`Ações de ${folderName}`} selecting={selecting} end={end}>
        <input
          type="checkbox"
          className="checkbox"
          aria-label={master === "all" ? "Desmarcar todas desta página" : "Selecionar todas desta página"}
          checked={master === "all"}
          ref={(el) => { if (el) el.indeterminate = master === "some"; }}
          disabled={state.status !== "ready" || state.items.length === 0}
          onChange={props.onCheckAll}
        />
        {selecting ? (
          <>
            <span className="toolbar__count" aria-live="polite">{checked.size === 1 ? "1 selecionada" : `${checked.size.toLocaleString("pt-BR")} selecionadas`}</span>
            <IconButton icon="x" label="Limpar seleção" data-tip="Limpar seleção (Esc)" compact onClick={props.onClearChecked} />
          </>
        ) : (
          <>
            <IconButton icon="refresh" label="Atualizar" onClick={props.onRefresh} disabled={searching || loading || syncing || !canRefresh} />
            {syncing && <Spinner size="sm" label="Sincronizando" />}
          </>
        )}
      </Toolbar>
      <div className="app-shell__list-body">
        {notice && <div style={{ padding: "var(--space-150) var(--space-200) 0" }}>{notice}</div>}
        {state.status === "loading" && (
          <ThreadList label="Carregando conversas" busy density={props.density}>
            {SKELETON_ROWS.map(([from, subject], i) => <ThreadSkeleton key={i} from={from} subject={subject} />)}
          </ThreadList>
        )}
        {state.status === "error" && (
          <EmptyState
            art="error"
            title="Não foi possível carregar a pasta"
            description={state.error.message}
            actions={<Button appearance="primary" onClick={props.onRefresh}>Tentar de novo</Button>}
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
          <EmailList
            label={folderName}
            items={state.items}
            density={props.density}
            openId={openId}
            checked={checked}
            showFolder={!!searching}
            renderBadge={renderBadge}
            onOpen={props.onOpen}
            onCheck={props.onCheck}
            onCheckAll={props.onCheckAll}
            onClearChecked={props.onClearChecked}
            canDrag={props.canDrag}
            onDragMessages={props.onDragMessages}
            onDragOver={props.onDragOver}
            onDropMessages={props.onDropMessages}
            onToggleStar={props.onToggleStar}
            onToggleRead={props.onToggleRead}
          />
        )}
      </div>
    </section>
  );
}

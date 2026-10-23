// Composição de DS: threads.html (.toolbar, .thread-list, .thread), carregamento.html (skeleton), vazios.html (.empty)
import { useRef, type KeyboardEvent } from "react";
import { formatListDate, senderLabel } from "../../lib/format";
import type { AppError, MessageHeader } from "../../lib/tauri";
import { Button, IconButton } from "../../ui/Button";
import { EmptyState } from "../../ui/EmptyState";
import { ThreadList, ThreadRow, ThreadSkeleton } from "../../ui/Thread";
import { Toolbar } from "../../ui/Toolbar";

export type InboxState =
  | { status: "loading" }
  | { status: "ready"; messages: MessageHeader[] }
  | { status: "error"; error: AppError };

const SKELETON_ROWS: Array<[string, string]> = [["40%", "70%"], ["30%", "55%"], ["45%", "65%"], ["35%", "80%"], ["25%", "60%"], ["40%", "50%"]];

interface InboxListProps {
  state: InboxState;
  selectedUid: number | null;
  onSelect: (uid: number) => void;
  onRefresh: () => void;
}

export function InboxList({ state, selectedUid, onSelect, onRefresh }: InboxListProps) {
  const rows = useRef(new Map<number, HTMLDivElement>());
  const loading = state.status === "loading";
  const count = state.status === "ready" ? state.messages.length : 0;

  function onRowKeyDown(event: KeyboardEvent<HTMLDivElement>, index: number) {
    if (state.status !== "ready") return;
    const { messages } = state;
    const move = (to: number) => {
      const target = messages[Math.max(0, Math.min(messages.length - 1, to))];
      onSelect(target.uid);
      rows.current.get(target.uid)?.focus();
    };
    if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(messages[index].uid); }
    else if (event.key === "j" || event.key === "ArrowDown") { event.preventDefault(); move(index + 1); }
    else if (event.key === "k" || event.key === "ArrowUp") { event.preventDefault(); move(index - 1); }
  }

  return (
    <>
      <Toolbar
        label="Ações da caixa de entrada"
        end={state.status === "ready" && count > 0 && <span>1–{count} mais recentes</span>}
      >
        <IconButton icon="refresh" label="Atualizar" onClick={onRefresh} disabled={loading} />
      </Toolbar>
      <div className="app-shell__list-body">
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
        {state.status === "ready" && count === 0 && (
          <EmptyState art="done" title="Tudo em dia" description="Não há mensagens na Caixa de entrada." />
        )}
        {state.status === "ready" && count > 0 && (
          <ThreadList label="Caixa de entrada">
            {state.messages.map((m, index) => (
              <ThreadRow
                key={m.uid}
                ref={(el) => { if (el) rows.current.set(m.uid, el); else rows.current.delete(m.uid); }}
                from={senderLabel(m.fromName, m.fromAddress)}
                subject={m.subject || "(sem assunto)"}
                date={formatListDate(m.date)}
                unread={!m.flags.seen}
                starred={m.flags.flagged}
                open={m.uid === selectedUid}
                onOpen={() => onSelect(m.uid)}
                onKeyDown={(e) => onRowKeyDown(e, index)}
              />
            ))}
          </ThreadList>
        )}
      </div>
    </>
  );
}

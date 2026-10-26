import type { ConversationMessage, ThreadSummary } from "./tauri";

/** Com mais recolhidas seguidas que isso, as do meio viram "N mensagens anteriores" (US8, D2). */
export const MAX_COLLAPSED_RUN = 4;

export type ConversationPart =
  | { kind: "open"; message: ConversationMessage }
  | { kind: "collapsed"; message: ConversationMessage }
  | { kind: "more"; count: number };

/**
 * A pilha de cards da conversa (D2): abertas a mais recente, as não lidas e as que o usuário abriu;
 * as outras recolhidas numa linha. Uma sequência de mais de 4 recolhidas mostra só a primeira e a
 * última, com "N mensagens anteriores" no meio, até `showAll`.
 */
export function conversationLayout(messages: ConversationMessage[], open: ReadonlySet<string>, showAll: boolean): ConversationPart[] {
  const parts: ConversationPart[] = [];
  let run: ConversationMessage[] = [];
  const flush = () => {
    if (!showAll && run.length > MAX_COLLAPSED_RUN) {
      parts.push({ kind: "collapsed", message: run[0] }, { kind: "more", count: run.length - 2 }, { kind: "collapsed", message: run[run.length - 1] });
    } else {
      for (const message of run) parts.push({ kind: "collapsed", message });
    }
    run = [];
  };
  messages.forEach((message, i) => {
    if (i === messages.length - 1 || open.has(message.id)) {
      flush();
      parts.push({ kind: "open", message });
    } else {
      run.push(message);
    }
  });
  flush();
  return parts;
}

/** Abertas ao carregar: a mais recente, as não lidas e a escolhida na lista. */
export function initiallyOpen(messages: ConversationMessage[], selectedId: string): Set<string> {
  const open = new Set(messages.filter((m) => !m.flags.seen).map((m) => m.id));
  open.add(selectedId);
  if (messages.length > 0) open.add(messages[messages.length - 1].id);
  return open;
}

const firstName = (name: string) => name.trim().split(/\s+/)[0] ?? name;

/** Quem participa, na linha da lista (L48): "Marina, Rafael, Paulo" ou "Paulo Henrique, você". */
export function participantsLabel(thread: ThreadSummary, fallback: string): string {
  const people = thread.participants.filter((p) => p.mine || p.name.trim());
  if (people.length === 0) return fallback;
  const short = people.length > 2;
  const names = people.slice(0, 3).map((p) => (p.mine ? "você" : short ? firstName(p.name) : p.name.trim()));
  const label = names.join(", ");
  return label.charAt(0).toUpperCase() + label.slice(1);
}

/** Cabeçalho que abre o texto citado: "Em 24 de out. de 2026, às 18:10, Marina escreveu:" ou "On … wrote:". */
const QUOTE_HEADER = /^\s*(Em .+ escreveu:|On .+ wrote:)\s*$/i;

/**
 * Divide um corpo de texto puro no que a pessoa escreveu e no que ela repete das anteriores (D3):
 * a partir do cabeçalho "Em …, … escreveu:" ou do bloco final de linhas com ">". Sem texto próprio,
 * nada fica escondido.
 */
export function splitQuote(text: string): { own: string; quote: string | null } {
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  let start = lines.findIndex((line) => QUOTE_HEADER.test(line));
  if (start < 0) {
    let i = lines.length;
    while (i > 0 && (lines[i - 1].trim() === "" || lines[i - 1].trimStart().startsWith(">"))) i--;
    start = lines.slice(i).some((line) => line.trimStart().startsWith(">")) ? i : -1;
  }
  if (start < 0) return { own: text, quote: null };
  const own = lines.slice(0, start).join("\n").trimEnd();
  if (own.trim() === "") return { own: text, quote: null };
  return { own, quote: lines.slice(start).join("\n").trim() };
}

/** O documento do e-mail com o texto citado à mostra: tira a regra que o esconde (marcada pelo Rust). */
export function revealQuote(html: string): string {
  return html.replace(/\/\*quote\*\/[\s\S]*?\/\*quote\*\//, "");
}

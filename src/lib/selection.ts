import type { MessageHeader } from "./tauri";

/**
 * Seleção múltipla da lista (US5, CA1): conjunto de ids marcados e a âncora do último clique,
 * para marcar intervalos com Shift. Funções puras; o Mailbox guarda o estado.
 */
export interface Selection {
  ids: ReadonlySet<string>;
  anchor: string | null;
}

export const emptySelection: Selection = { ids: new Set(), anchor: null };

/** Marca ou desmarca uma linha. Com `range`, marca o intervalo desde a âncora até ela. */
export function toggle(selection: Selection, order: readonly string[], id: string, range: boolean): Selection {
  const ids = new Set(selection.ids);
  const from = selection.anchor ? order.indexOf(selection.anchor) : -1;
  const to = order.indexOf(id);
  if (range && from >= 0 && to >= 0) {
    const [a, b] = from < to ? [from, to] : [to, from];
    for (const each of order.slice(a, b + 1)) ids.add(each);
    return { ids, anchor: id };
  }
  if (ids.has(id)) ids.delete(id);
  else ids.add(id);
  return { ids, anchor: id };
}

/** Checkbox da toolbar: marca a página inteira, ou desmarca tudo se ela já estiver toda marcada. */
export function toggleAll(selection: Selection, order: readonly string[]): Selection {
  if (order.length > 0 && order.every((id) => selection.ids.has(id))) return emptySelection;
  return { ids: new Set(order), anchor: selection.anchor };
}

/** Estado do checkbox da toolbar. */
export function masterState(selection: Selection, order: readonly string[]): "none" | "some" | "all" {
  const count = order.filter((id) => selection.ids.has(id)).length;
  if (count === 0) return "none";
  return count === order.length ? "all" : "some";
}

/** Mantém só os ids que continuam na lista (depois de mover, excluir ou recarregar). */
export function prune(selection: Selection, order: readonly string[]): Selection {
  const present = new Set(order);
  const ids = [...selection.ids].filter((id) => present.has(id));
  if (ids.length === selection.ids.size) return selection;
  return { ids: new Set(ids), anchor: selection.anchor && present.has(selection.anchor) ? selection.anchor : null };
}

/** Em quem os comandos agem (CA2): as marcadas, na ordem da lista; sem marcadas, a aberta. */
export function commandTargets(selection: Selection, items: readonly MessageHeader[], openId: string | null): MessageHeader[] {
  if (selection.ids.size > 0) return items.filter((m) => selection.ids.has(m.id));
  const open = items.find((m) => m.id === openId);
  return open ? [open] : [];
}

/** Lida/Não lida em seleção mista: se alguma estiver não lida, a ação é marcar como lida. */
export function readActionFor(messages: readonly MessageHeader[]): { seen: boolean } {
  return { seen: messages.some((m) => !m.flags.seen) };
}

/** "1 conversa", "3 conversas". */
export function conversations(count: number): string {
  return count === 1 ? "1 conversa" : `${count.toLocaleString("pt-BR")} conversas`;
}

/** Texto do toast de uma ação em lote: "Mensagem arquivada" / "3 conversas arquivadas". */
export function batchTitle(count: number, single: string, plural: (what: string) => string): string {
  return count === 1 ? single : plural(conversations(count));
}

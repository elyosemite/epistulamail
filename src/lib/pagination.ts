/**
 * Paginação por cursor (US2, CA2). O backend não guarda estado: a pilha de cursores das páginas
 * já visitadas fica aqui, para "Página anterior" voltar exatamente ao que foi visto.
 */
export interface Pager {
  /** Cursor de cada página visitada; `null` é a primeira página. */
  stack: (string | null)[];
  /** Cursor da página seguinte, informado pelo backend; `null` quando não há mais. */
  next: string | null;
}

export const PAGE_SIZE = 50;

export const firstPage = (): Pager => ({ stack: [null], next: null });

export const currentCursor = (pager: Pager): string | null => pager.stack[pager.stack.length - 1];

export const pageIndex = (pager: Pager): number => pager.stack.length - 1;

export const canGoBack = (pager: Pager): boolean => pager.stack.length > 1;

export function withNext(pager: Pager, next: string | null): Pager {
  return { ...pager, next };
}

export function goForward(pager: Pager): Pager {
  return pager.next ? { stack: [...pager.stack, pager.next], next: null } : pager;
}

export function goBack(pager: Pager): Pager {
  return canGoBack(pager) ? { stack: pager.stack.slice(0, -1), next: null } : pager;
}

/** Texto da toolbar no padrão de threads.html: "1–50 de 1.284". */
export function rangeLabel(pager: Pager, count: number, total: number): string {
  if (count === 0) return `0 de ${total.toLocaleString("pt-BR")}`;
  const start = pageIndex(pager) * PAGE_SIZE + 1;
  return `${start.toLocaleString("pt-BR")}–${(start + count - 1).toLocaleString("pt-BR")} de ${total.toLocaleString("pt-BR")}`;
}

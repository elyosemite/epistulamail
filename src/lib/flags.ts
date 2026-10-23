import type { Folder, MessageFlags, MessageHeader } from "./tauri";

/**
 * Atualização otimista de flags (US2, CA5): aplica na hora e guarda o estado anterior para
 * reverter se o servidor recusar.
 */
export interface FlagChange {
  ids: string[];
  change: Partial<MessageFlags>;
}

export function applyFlags(items: MessageHeader[], ids: string[], change: Partial<MessageFlags>): MessageHeader[] {
  const set = new Set(ids);
  return items.map((m) => (set.has(m.id) ? { ...m, flags: { ...m.flags, ...change } } : m));
}

export function restoreFlags(items: MessageHeader[], previous: Map<string, MessageFlags>): MessageHeader[] {
  return items.map((m) => {
    const flags = previous.get(m.id);
    return flags ? { ...m, flags } : m;
  });
}

export function snapshotFlags(items: MessageHeader[], ids: string[]): Map<string, MessageFlags> {
  const set = new Set(ids);
  return new Map(items.filter((m) => set.has(m.id)).map((m) => [m.id, m.flags]));
}

/** Quanto o contador de não lidas da pasta muda com a mudança de `seen`. */
export function unreadDelta(previous: Map<string, MessageFlags>, change: Partial<MessageFlags>): number {
  if (change.seen === undefined) return 0;
  let delta = 0;
  for (const flags of previous.values()) {
    if (flags.seen !== change.seen) delta += change.seen ? -1 : 1;
  }
  return delta;
}

export function adjustUnread(folders: Folder[], folderId: string, delta: number): Folder[] {
  if (delta === 0) return folders;
  return folders.map((f) => (f.id === folderId && f.unread !== null ? { ...f, unread: Math.max(0, f.unread + delta) } : f));
}

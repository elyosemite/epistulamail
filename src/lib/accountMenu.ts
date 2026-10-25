// Textos da seção 1 do menu da conta (US7, CA6). Nenhum cita o provedor da conta.
import { formatBytes } from "./format";

/** "Sincronizado agora", "há 2 min", "há 3 h" ou a data. `lastSyncAt` em segundos Unix. */
export function syncedLabel(lastSyncAt: number | null, now: number = Date.now()): string {
  if (lastSyncAt === null) return "Ainda não sincronizado";
  const seconds = Math.max(0, Math.round(now / 1000 - lastSyncAt));
  if (seconds < 60) return "Sincronizado agora";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `Sincronizado há ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `Sincronizado há ${hours} h`;
  const date = new Date(lastSyncAt * 1000).toLocaleDateString("pt-BR", { day: "numeric", month: "short" }).replace(".", "");
  return `Sincronizado em ${date}`;
}

export const unreadLabel = (n: number) => (n === 0 ? "Nenhuma não lida" : n === 1 ? "1 não lida" : `${n.toLocaleString("pt-BR")} não lidas`);

export function savedLabel(messages: number, bytes: number): string {
  const count = messages === 1 ? "1 mensagem salva" : `${messages.toLocaleString("pt-BR")} mensagens salvas`;
  return `${count} neste computador · ${formatBytes(bytes)}`;
}

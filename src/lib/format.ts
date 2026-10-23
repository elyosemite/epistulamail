const MONTHS = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

const pad = (n: number) => String(n).padStart(2, "0");

/** Data curta da lista, no padrão de threads.html: "10:42" hoje, "20 out" no ano, "20 out 2025" antes. */
export function formatListDate(iso: string, now: Date = new Date()): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  if (date.toDateString() === now.toDateString()) return `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  const dayMonth = `${date.getDate()} ${MONTHS[date.getMonth()]}`;
  return date.getFullYear() === now.getFullYear() ? dayMonth : `${dayMonth} ${date.getFullYear()}`;
}

/** Data completa do painel de leitura, no padrão de leitura.html: "Hoje, 10:42" ou "20 out 2026, 10:42". */
export function formatFullDate(iso: string, now: Date = new Date()): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  if (date.toDateString() === now.toDateString()) return `Hoje, ${time}`;
  return `${date.getDate()} ${MONTHS[date.getMonth()]} ${date.getFullYear()}, ${time}`;
}

export function initials(name: string): string {
  const parts = name.replace(/[<>"@].*$/, "").trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  const first = parts[0][0] ?? "";
  const last = parts.length > 1 ? parts[parts.length - 1][0] : "";
  return (first + last).toUpperCase();
}

/** Cores de avatar disponíveis em identidade.html, escolhidas de forma estável pelo endereço. */
export const AVATAR_COLORS = ["", "teal", "green", "purple", "magenta", "orange"] as const;
export type AvatarColor = (typeof AVATAR_COLORS)[number];

export function avatarColor(seed: string): AvatarColor {
  let hash = 0;
  for (const ch of seed) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  return AVATAR_COLORS[Math.abs(hash) % AVATAR_COLORS.length];
}

export function senderLabel(name: string, address: string): string {
  return name.trim() || address || "(remetente desconhecido)";
}

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

export function formatPeople(people: { name: string; address: string }[]): string {
  return people.map((p) => (p.name && p.address ? `${p.name} <${p.address}>` : p.name || p.address)).join(", ");
}

/** Resumo de destinatários do botão `.message__to`: "para Ana, b@y.com". */
export function recipientsSummary(people: { name: string; address: string }[]): string {
  return people.length === 0 ? "sem destinatários" : `para ${people.map((p) => p.name || p.address).join(", ")}`;
}

/** Parágrafos de texto puro: blocos separados por linha em branco, linhas simples preservadas. */
export function paragraphs(text: string): string[][] {
  return text
    .replace(/\r\n?/g, "\n")
    .split(/\n\s*\n/)
    .map((block) => block.split("\n").map((line) => line.trimEnd()))
    .filter((lines) => lines.some((line) => line.trim() !== ""));
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = value < 10 && unit > 0 ? 1 : 0;
  return `${value.toLocaleString("pt-BR", { maximumFractionDigits: digits })} ${units[unit]}`;
}

/** Variante de ícone de anexo publicada em anexos.html. */
export function attachmentKind(mimeType: string): "pdf" | "img" | "doc" | "other" {
  if (mimeType === "application/pdf") return "pdf";
  if (mimeType.startsWith("image/")) return "img";
  if (/word|document|msword|rtf|opendocument\.text/.test(mimeType)) return "doc";
  return "other";
}

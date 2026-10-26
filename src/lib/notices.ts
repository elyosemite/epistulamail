import { formatListDate } from "./format";
import type { NoticeKind } from "./tauri";

/** Contador do sino (CA7): até "9+"; sem nenhuma, nada. */
export function bellCount(unseen: number): string | null {
  if (unseen <= 0) return null;
  return unseen > 9 ? "9+" : String(unseen);
}

export function bellLabel(unseen: number, quiet: boolean): string {
  const base = quiet ? "Notificações, Não perturbe ligado" : "Notificações";
  if (unseen <= 0) return base;
  return `${base}, ${unseen > 9 ? "mais de 9" : unseen} não ${unseen === 1 ? "vista" : "vistas"}`;
}

/** Hora da notificação no painel: "09:15" hoje, "Ontem", ou "20 out". */
export function noticeTime(createdAt: number, now: Date = new Date()): string {
  const date = new Date(createdAt * 1000);
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (date.toDateString() === yesterday.toDateString()) return "Ontem";
  return formatListDate(date.toISOString(), now);
}

/** Ícone de cada tipo de aviso, num círculo neutro, vermelho (falha) ou amarelo (atenção). */
export function noticeIcon(kind: NoticeKind): { icon: string; tone: "neutral" | "danger" | "warning" } {
  switch (kind) {
    case "failure":
      return { icon: "error", tone: "danger" };
    case "reauth":
      return { icon: "log-in", tone: "warning" };
    case "due":
      return { icon: "flag", tone: "neutral" };
    case "offline":
      return { icon: "globe", tone: "neutral" };
    case "summary":
      return { icon: "inbox", tone: "neutral" };
    case "info":
      return { icon: "bell", tone: "neutral" };
    default:
      return { icon: "mail", tone: "neutral" };
  }
}

/** O texto da linha de baixo do painel: as duas linhas da notificação numa só. */
export function noticeText(line1: string, line2: string): string {
  return [line1, line2].filter((s) => s.trim()).join(" · ");
}

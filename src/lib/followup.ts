import type { Followup } from "./tauri";

/** Opções do menu "Sinalizar" (US4, CA7). */
export type FollowupChoice = "today" | "tomorrow" | "this_week" | "next_week" | "no_date";

const WEEKDAYS = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"];
const MONTHS = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

const pad = (n: number) => String(n).padStart(2, "0");

/** Data local em AAAA-MM-DD. */
export function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function addDays(d: Date, days: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + days);
}

function parseIso(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/**
 * Prazo de cada opção. "Esta semana" é a sexta desta semana (ou hoje, se já for sexta ou fim de
 * semana); "Próxima semana" é a segunda seguinte.
 */
export function dueFor(choice: FollowupChoice, today: Date = new Date()): string | null {
  const day = today.getDay();
  switch (choice) {
    case "today":
      return isoDate(today);
    case "tomorrow":
      return isoDate(addDays(today, 1));
    case "this_week":
      return isoDate(addDays(today, day >= 5 || day === 0 ? 0 : 5 - day));
    case "next_week":
      return isoDate(addDays(today, ((8 - day) % 7) || 7));
    case "no_date":
      return null;
  }
}

/** "sex, 24 out". */
export function shortDate(iso: string): string {
  const d = parseIso(iso);
  return `${WEEKDAYS[d.getDay()]}, ${d.getDate()} ${MONTHS[d.getMonth()]}`;
}

export type DueTone = "today" | "future" | "overdue" | "done" | "none";

/** Texto e tom do lozenge da mensagem sinalizada ("Hoje", "Amanhã", "sex, 24 out", "Atrasada"). */
export function dueLabel(followup: Followup | null, today: Date = new Date()): { text: string; tone: DueTone } | null {
  if (!followup) return null;
  if (followup.completed) return { text: "Concluída", tone: "done" };
  if (!followup.due) return null;
  const t = isoDate(today);
  if (followup.due < t) return { text: "Atrasada", tone: "overdue" };
  if (followup.due === t) return { text: "Hoje", tone: "today" };
  if (followup.due === isoDate(addDays(today, 1))) return { text: "Amanhã", tone: "future" };
  return { text: shortDate(followup.due), tone: "future" };
}

export const FOLLOWUP_CHOICES: { choice: FollowupChoice; label: string }[] = [
  { choice: "today", label: "Hoje" },
  { choice: "tomorrow", label: "Amanhã" },
  { choice: "this_week", label: "Esta semana" },
  { choice: "next_week", label: "Próxima semana" },
  { choice: "no_date", label: "Sem data" },
];

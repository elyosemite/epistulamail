// Senha do Epistula e bloqueio (US10, CA2 e CA3): regras que a interface usa.
import type { LockMinutes } from "./tauri";

export const LOCK_CHOICES: Array<{ value: LockMinutes; label: string }> = [
  { value: "5", label: "5 minutos" },
  { value: "15", label: "15 minutos" },
  { value: "30", label: "30 minutos" },
  { value: "60", label: "1 hora" },
  { value: "never", label: "Nunca" },
];

/** Mesmo mínimo do backend (`lock::MIN_PASSWORD_CHARS`). */
export const MIN_PASSWORD_CHARS = 8;

/** Tempo sem uso até bloquear, em ms; `null` = nunca. */
export function idleLimitMs(minutes: LockMinutes): number | null {
  return minutes === "never" ? null : Number(minutes) * 60_000;
}

/** Senha nova curta demais: o erro, ou `undefined`. */
export function tooShort(password: string): string | undefined {
  return [...password].length < MIN_PASSWORD_CHARS ? `Use pelo menos ${MIN_PASSWORD_CHARS} caracteres.` : undefined;
}

/** A repetição não confere: o erro, ou `undefined`. */
export function mismatch(password: string, repeat: string): string | undefined {
  return password !== repeat ? "As duas senhas não são iguais." : undefined;
}

/** "Senha incorreta. Tente de novo em 8 segundos." */
export function retryLabel(seconds: number): string {
  if (seconds <= 0) return "Senha incorreta.";
  if (seconds >= 60) {
    const minutes = Math.ceil(seconds / 60);
    return `Senha incorreta. Tente de novo em ${minutes} ${minutes === 1 ? "minuto" : "minutos"}.`;
  }
  return `Senha incorreta. Tente de novo em ${seconds} ${seconds === 1 ? "segundo" : "segundos"}.`;
}

/** Atalho para bloquear: Ctrl+Shift+L. */
export function isLockShortcut(e: { ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean; key: string }): boolean {
  return (e.ctrlKey || e.metaKey) && e.shiftKey && !e.altKey && e.key.toLowerCase() === "l";
}

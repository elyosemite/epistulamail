import type { SyncMinutes } from "./tauri";

/** Intervalos oferecidos (US5, D3). `null` = manualmente. Iguais aos aceitos pelo backend. */
export const SYNC_CHOICES: readonly SyncMinutes[] = [null, 1, 2, 5, 10, 15, 30, 60];

export function intervalLabel(minutes: SyncMinutes): string {
  if (minutes === null) return "Manualmente";
  if (minutes === 60) return "1 hora";
  return minutes === 1 ? "1 minuto" : `${minutes} minutos`;
}

/** Forma curta para "Usar o padrão (5 min)". */
export function intervalShort(minutes: SyncMinutes): string {
  if (minutes === null) return "manual";
  return minutes === 60 ? "1 h" : `${minutes} min`;
}

/** Valor de `<option>`: "manual" ou os minutos; "default" volta a seguir o padrão. */
export const encodeChoice = (minutes: SyncMinutes) => (minutes === null ? "manual" : String(minutes));

export function decodeChoice(value: string): { useDefault: true } | { useDefault: false; minutes: SyncMinutes } {
  if (value === "default") return { useDefault: true };
  if (value === "manual") return { useDefault: false, minutes: null };
  const minutes = Number(value);
  if (!SYNC_CHOICES.includes(minutes)) throw new Error("intervalo inválido");
  return { useDefault: false, minutes };
}

import { dueFor, isoDate } from "./followup";
import type { FlagBucket, FlaggedCounts } from "./tauri";

/** Grupos de "Sinalizadas" na coluna de pastas, na ordem da coluna (US6, CA6). */
export const FLAG_BUCKETS: ReadonlyArray<{ bucket: FlagBucket; label: string; key: keyof FlaggedCounts }> = [
  { bucket: "overdue", label: "Atrasadas", key: "overdue" },
  { bucket: "today", label: "Hoje", key: "today" },
  { bucket: "this_week", label: "Esta semana", key: "thisWeek" },
  { bucket: "later", label: "Mais tarde", key: "later" },
  { bucket: "no_date", label: "Sem data", key: "noDate" },
];

/** Datas de referência na data local: "Esta semana" termina no mesmo dia que o prazo da US4 (sexta). */
export function flagDates(now: Date = new Date()): { today: string; weekEnd: string } {
  return { today: isoDate(now), weekEnd: dueFor("this_week", now) ?? isoDate(now) };
}

/** Só os prazos que têm mensagens; vazio esconde o grupo. */
export function visibleBuckets(counts: FlaggedCounts | null) {
  if (!counts) return [];
  return FLAG_BUCKETS.filter((b) => counts[b.key] > 0).map((b) => ({ ...b, count: counts[b.key] }));
}

export const bucketLabel = (bucket: FlagBucket) => FLAG_BUCKETS.find((b) => b.bucket === bucket)?.label ?? "Sinalizadas";

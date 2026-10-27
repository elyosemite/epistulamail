// Busca (US16): chips de pessoa, período e filtros, operadores digitados e a consulta que vai ao backend.
import { actionFor, SPEC_VERSION } from "./rules";
import type { RuleCondition, RuleSpec, SearchQuery } from "./tauri";

export type PersonRole = "from" | "to" | "cc" | "with";

export type SearchChip =
  | { kind: "person"; role: PersonRole; name: string; address: string | null }
  | { kind: "period"; after: string | null; before: string | null; label: string }
  | { kind: "attachment" }
  | { kind: "unread" }
  | { kind: "starred" }
  | { kind: "folder"; id: string; name: string };

export interface Search {
  chips: SearchChip[];
  text: string;
  allAccounts: boolean;
  oldestFirst: boolean;
}

export const EMPTY_SEARCH: Search = { chips: [], text: "", allAccounts: false, oldestFirst: false };

/** Há o que buscar: algum chip, ou pelo menos 2 letras de texto. */
export function hasCriteria(search: Search): boolean {
  return search.chips.length > 0 || search.text.trim().length >= 2;
}

/** O rótulo do papel no chip de pessoa ("De", "Para", "Cc"); "com" não tem rótulo. */
export const ROLE_LABEL: Record<PersonRole, string | null> = { from: "De", to: "Para", cc: "Cc", with: null };

/** Operadores digitados e o que cada um vira. */
const PERSON_OPS: Record<string, PersonRole> = { de: "from", from: "from", para: "to", to: "to", cc: "cc", com: "with" };

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

export function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

const MONTHS = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

/** "2026-03-31" → "31 mar 2026". */
export function shortDate(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return `${d} ${MONTHS[m - 1]} ${y}`;
}

/** Data digitada: "2026-03-31", "31/03/2026" ou "31/3/26". */
export function parseDate(value: string): string | null {
  const v = value.trim();
  let y: number, m: number, d: number;
  const iso = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(v);
  const br = /^(\d{1,2})\/(\d{1,2})\/(\d{2}|\d{4})$/.exec(v);
  if (iso) [y, m, d] = [Number(iso[1]), Number(iso[2]), Number(iso[3])];
  else if (br) [d, m, y] = [Number(br[1]), Number(br[2]), Number(br[3].length === 2 ? `20${br[3]}` : br[3])];
  else return null;
  const date = new Date(y, m - 1, d);
  if (date.getFullYear() !== y || date.getMonth() !== m - 1 || date.getDate() !== d) return null;
  return isoDate(date);
}

/** Rótulo de um intervalo: "1 jan a 31 mar 2026", "Depois de 1 jan 2026", "Em 2025". */
export function periodLabel(after: string | null, before: string | null): string {
  if (after && before) {
    if (after.endsWith("-01-01") && before.endsWith("-12-31") && after.slice(0, 4) === before.slice(0, 4)) return `Em ${after.slice(0, 4)}`;
    const [a, b] = [shortDate(after), shortDate(before)];
    return after.slice(0, 4) === before.slice(0, 4) ? `${a.replace(/ \d{4}$/, "")} a ${b}` : `${a} a ${b}`;
  }
  if (after) return `Depois de ${shortDate(after)}`;
  if (before) return `Antes de ${shortDate(before)}`;
  return "Qualquer data";
}

export type PeriodPreset = "today" | "7d" | "30d" | "year";

export const PERIOD_PRESETS: { id: PeriodPreset; label: string }[] = [
  { id: "today", label: "Hoje" },
  { id: "7d", label: "Últimos 7 dias" },
  { id: "30d", label: "Últimos 30 dias" },
  { id: "year", label: "Este ano" },
];

export function presetChip(preset: PeriodPreset, now: Date = new Date()): SearchChip {
  const today = isoDate(now);
  const back = (days: number) => isoDate(new Date(now.getFullYear(), now.getMonth(), now.getDate() - days));
  const label = PERIOD_PRESETS.find((p) => p.id === preset)!.label;
  switch (preset) {
    case "today":
      return { kind: "period", after: today, before: today, label };
    case "7d":
      return { kind: "period", after: back(6), before: null, label };
    case "30d":
      return { kind: "period", after: back(29), before: null, label };
    case "year":
      return { kind: "period", after: `${now.getFullYear()}-01-01`, before: null, label };
  }
}

/** O operador que está sendo digitado no fim do texto ("de: yu" → pessoa "yu"), para as sugestões. */
export type Typing = { kind: "person"; role: PersonRole; term: string } | { kind: "folder"; term: string } | null;

const TRAILING_OP = /(?:^|\s)(de|from|para|to|cc|com|na)\s*:\s*([^:]*)$/i;

export function typingAt(text: string): Typing {
  const m = TRAILING_OP.exec(text);
  if (!m) return null;
  const op = m[1].toLowerCase();
  if (op === "na") return { kind: "folder", term: m[2].trim() };
  return { kind: "person", role: PERSON_OPS[op], term: m[2].trim() };
}

/** Tira do texto o operador que virou chip. */
export function withoutTrailingOp(text: string): string {
  return text.replace(TRAILING_OP, "").trim();
}

/**
 * Operadores completos no texto (Enter): pessoas (sem endereço, pelo que foi digitado), datas e filtros
 * viram chips; o resto fica como texto. Devolve os chips novos e o texto que sobra.
 */
export function parseOperators(text: string): { chips: SearchChip[]; text: string } {
  const chips: SearchChip[] = [];
  let rest = text;
  const take = (re: RegExp, make: (m: RegExpExecArray) => SearchChip | null) => {
    rest = rest.replace(re, (...args) => {
      const m = args.slice(0, -2) as unknown as RegExpExecArray;
      const chip = make(m);
      if (!chip) return m[0];
      chips.push(chip);
      return " ";
    });
  };
  take(/(?:^|\s)tem\s*:\s*anexos?\b/gi, () => ({ kind: "attachment" }));
  take(/(?:^|\s)(?:é|e)\s*:\s*n[ãa]o-?lidas?\b/gi, () => ({ kind: "unread" }));
  take(/(?:^|\s)(?:é|e)\s*:\s*estrela\b/gi, () => ({ kind: "starred" }));
  take(/(?:^|\s)em\s*:\s*(\d{4})\b/gi, (m) => ({ kind: "period", after: `${m[1]}-01-01`, before: `${m[1]}-12-31`, label: `Em ${m[1]}` }));
  let after: string | null = null;
  let before: string | null = null;
  take(/(?:^|\s)depois\s*:\s*(\S+)/gi, (m) => {
    after = parseDate(m[1]);
    return after ? { kind: "period", after, before: null, label: "" } : null;
  });
  take(/(?:^|\s)antes\s*:\s*(\S+)/gi, (m) => {
    before = parseDate(m[1]);
    return before ? { kind: "period", after: null, before, label: "" } : null;
  });
  // "depois:" e "antes:" juntos são um intervalo só.
  const loose = chips.filter((c) => c.kind === "period" && c.label === "");
  if (loose.length > 0) {
    const merged: SearchChip = { kind: "period", after, before, label: periodLabel(after, before) };
    chips.splice(0, chips.length, ...chips.filter((c) => !loose.includes(c)), merged);
  }
  // Pessoa: o operador vai até o fim do texto ("de: Yuri Melo").
  const person = /(?:^|\s)(de|from|para|to|cc|com)\s*:\s*(.+)$/i.exec(rest);
  if (person && person[2].trim()) {
    const term = person[2].trim();
    const address = /^[^\s@]+@[^\s@]+$/.test(term) ? term.toLowerCase() : null;
    chips.push({ kind: "person", role: PERSON_OPS[person[1].toLowerCase()], name: address ? "" : term, address });
    rest = rest.slice(0, person.index);
  }
  return { chips, text: rest.replace(/\s+/g, " ").trim() };
}

/** Acrescenta um chip: um período substitui o anterior; filtros e pessoas iguais não se repetem. */
export function addChip(chips: SearchChip[], chip: SearchChip): SearchChip[] {
  if (chip.kind === "period") return [...chips.filter((c) => c.kind !== "period"), chip];
  if (chip.kind === "folder") return [...chips.filter((c) => c.kind !== "folder"), chip];
  if (chips.some((c) => sameChip(c, chip))) return chips;
  return [...chips, chip];
}

function sameChip(a: SearchChip, b: SearchChip): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "person" && b.kind === "person") return a.role === b.role && (a.address ?? a.name).toLowerCase() === (b.address ?? b.name).toLowerCase();
  return a.kind !== "person";
}

/** A consulta do backend a partir dos chips e do texto. */
export function toQuery(search: Search): SearchQuery {
  const people = (role: PersonRole) => search.chips.flatMap((c) => (c.kind === "person" && c.role === role ? [c.address ?? c.name] : []));
  const period = search.chips.find((c): c is Extract<SearchChip, { kind: "period" }> => c.kind === "period");
  const folder = search.chips.find((c): c is Extract<SearchChip, { kind: "folder" }> => c.kind === "folder");
  return {
    text: search.text.trim(),
    from: people("from"),
    to: people("to"),
    cc: people("cc"),
    with: people("with"),
    after: period?.after ?? null,
    before: period?.before ?? null,
    hasAttachment: search.chips.some((c) => c.kind === "attachment"),
    unread: search.chips.some((c) => c.kind === "unread"),
    starred: search.chips.some((c) => c.kind === "starred"),
    folder: search.allAccounts ? null : folder?.id ?? null,
    allAccounts: search.allAccounts,
    oldestFirst: search.oldestFirst,
  };
}

/** Texto de um chip, para a lista de recentes e o nome sugerido da busca salva. */
export function chipText(chip: SearchChip): string {
  switch (chip.kind) {
    case "person": {
      const who = chip.name || chip.address || "";
      const role = ROLE_LABEL[chip.role];
      return role ? `${role.toLowerCase()} ${who}` : `com ${who}`;
    }
    case "period":
      return chip.label.charAt(0).toLowerCase() + chip.label.slice(1);
    case "attachment":
      return "com anexo";
    case "unread":
      return "não lidas";
    case "starred":
      return "com estrela";
    case "folder":
      return `em ${chip.name}`;
  }
}

/** "contrato · de Yuri Melo · com anexo". */
export function searchLabel(search: Pick<Search, "chips" | "text">): string {
  return [search.text.trim(), ...search.chips.map(chipText)].filter(Boolean).join(" · ");
}

/** A busca de volta a partir da consulta guardada (recentes e salvas). */
export function fromQuery(q: SearchQuery, folderName = "Pasta"): Search {
  const chips: SearchChip[] = [];
  const people = (list: string[], role: PersonRole) =>
    list.forEach((v) => chips.push({ kind: "person", role, name: v.includes("@") ? "" : v, address: v.includes("@") ? v : null }));
  people(q.from, "from");
  people(q.to, "to");
  people(q.cc, "cc");
  people(q.with, "with");
  if (q.after || q.before) chips.push({ kind: "period", after: q.after, before: q.before, label: periodLabel(q.after, q.before) });
  if (q.hasAttachment) chips.push({ kind: "attachment" });
  if (q.unread) chips.push({ kind: "unread" });
  if (q.starred) chips.push({ kind: "starred" });
  if (q.folder) chips.push({ kind: "folder", id: q.folder, name: folderName });
  return { chips, text: q.text, allAccounts: q.allAccounts, oldestFirst: q.oldestFirst };
}

/**
 * "Criar uma regra com esta busca…": De, Para, Cc, Assunto, anexo, lida e estrela vindos dos chips. A regra junta tudo
 * com "e", então só a primeira pessoa de cada papel entra; "com" (qualquer papel) e o período não viram condição.
 */
export function ruleFromSearch(search: Pick<Search, "chips" | "text">): RuleSpec {
  const conditions: RuleCondition[] = [];
  const first = (role: PersonRole) => search.chips.find((c): c is Extract<SearchChip, { kind: "person" }> => c.kind === "person" && c.role === role);
  for (const role of ["from", "to", "cc"] as const) {
    const person = first(role);
    if (person) conditions.push({ field: role, op: "contains", value: person.address ?? person.name });
  }
  if (search.text.trim()) conditions.push({ field: "subject", op: "contains", value: search.text.trim() });
  if (search.chips.some((c) => c.kind === "attachment")) conditions.push({ field: "has_files", op: "yes", value: "" });
  if (search.chips.some((c) => c.kind === "unread")) conditions.push({ field: "seen", op: "no", value: "" });
  if (search.chips.some((c) => c.kind === "starred")) conditions.push({ field: "flagged", op: "yes", value: "" });
  return { v: SPEC_VERSION, folders: [], matchAll: true, conditions, exceptions: [], actions: [actionFor("move")], stop: false };
}

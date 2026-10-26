// Regras (US12): campos, operadores, ações, a frase de cada regra (L58), modelos (CA7) e validação.
import type { Folder, RuleAction, RuleCondition, RuleField, RuleOp, RuleRun, RuleSpec } from "./tauri";

export const SPEC_VERSION = 1;

type FieldKind = "text" | "bool" | "number";

interface FieldInfo {
  value: RuleField;
  label: string;
  kind: FieldKind;
  /** Rótulos de "sim" e "não" (campos de sim ou não) ou do "maior que" (números). */
  yes?: string;
  no?: string;
  unit?: string;
  /** Limpeza diária (CA8): não roda ao sincronizar. */
  cleanup?: boolean;
}

export const FIELDS: FieldInfo[] = [
  { value: "from", label: "De", kind: "text" },
  { value: "from_domain", label: "Domínio do remetente", kind: "text" },
  { value: "to", label: "Para", kind: "text" },
  { value: "cc", label: "Cc", kind: "text" },
  { value: "to_or_cc", label: "Para ou Cc", kind: "text" },
  { value: "only_to_me", label: "Enviado só para mim", kind: "bool", yes: "sim", no: "não" },
  { value: "subject", label: "Assunto", kind: "text" },
  { value: "snippet", label: "Trecho do texto", kind: "text" },
  { value: "list", label: "Lista de e-mails", kind: "bool", yes: "é de uma lista", no: "não é de uma lista" },
  { value: "has_files", label: "Arquivos anexados", kind: "bool", yes: "tem", no: "não tem" },
  { value: "size_over_kb", label: "Tamanho", kind: "number", yes: "maior que", unit: "KB" },
  { value: "seen", label: "Lida", kind: "bool", yes: "já lida", no: "não lida" },
  { value: "flagged", label: "Estrela", kind: "bool", yes: "com estrela", no: "sem estrela" },
  { value: "older_than_days", label: "Idade", kind: "number", yes: "mais antigo que", unit: "dias", cleanup: true },
  { value: "not_newest_from_sender", label: "Mais recente do remetente", kind: "bool", yes: "não é o mais recente", cleanup: true },
];

export const TEXT_OPS: Array<{ value: RuleOp; label: string }> = [
  { value: "contains", label: "contém" },
  { value: "not_contains", label: "não contém" },
  { value: "is", label: "é" },
  { value: "starts_with", label: "começa com" },
  { value: "ends_with", label: "termina com" },
];

export const fieldInfo = (field: RuleField): FieldInfo => FIELDS.find((f) => f.value === field)!;

/** Operadores que o campo aceita, com o rótulo de cada um. */
export function opsFor(field: RuleField): Array<{ value: RuleOp; label: string }> {
  const f = fieldInfo(field);
  if (f.kind === "text") return TEXT_OPS;
  if (f.kind === "number") return [{ value: "over", label: f.yes! }];
  return f.no ? [{ value: "yes", label: f.yes! }, { value: "no", label: f.no }] : [{ value: "yes", label: f.yes! }];
}

/** Condição nova para o campo, com o primeiro operador e um valor padrão. */
export function conditionFor(field: RuleField): RuleCondition {
  const f = fieldInfo(field);
  return { field, op: opsFor(field)[0].value, value: f.kind === "number" ? (field === "older_than_days" ? "30" : "1024") : "" };
}

export type ActionKind = RuleAction["kind"];

export const ACTIONS: Array<{ value: ActionKind; label: string; icon: string }> = [
  { value: "move", label: "Mover para", icon: "folder-move" },
  { value: "archive", label: "Arquivar", icon: "archive" },
  { value: "delete", label: "Excluir (Lixeira)", icon: "trash" },
  { value: "copy", label: "Aplicar marcador", icon: "tag" },
  { value: "mark_read", label: "Marcar como lida", icon: "mail-open" },
  { value: "star", label: "Marcar com estrela", icon: "star" },
  { value: "followup", label: "Sinalizar com prazo", icon: "flag" },
  { value: "silence", label: "Não notificar", icon: "bell-off" },
  { value: "notify_always", label: "Notificar sempre", icon: "bell" },
];

export const FOLLOWUP_DAYS = [
  { value: "0", label: "hoje" },
  { value: "1", label: "amanhã" },
  { value: "7", label: "em uma semana" },
];

const LEAVES = new Set<ActionKind>(["move", "archive", "delete"]);
export const leavesFolder = (a: RuleAction) => LEAVES.has(a.kind);

export function actionFor(kind: ActionKind): RuleAction {
  switch (kind) {
    case "move":
    case "copy":
      return { kind, folder: "" };
    case "followup":
      return { kind, days: 7 };
    default:
      return { kind } as RuleAction;
  }
}

/** Ações que ainda podem entrar na posição `index`: sem repetir e uma só que tira o e-mail da pasta. */
export function availableActions(actions: RuleAction[], index: number) {
  const others = actions.filter((_, i) => i !== index);
  const leaving = others.some(leavesFolder);
  return ACTIONS.filter((a) => !others.some((o) => o.kind === a.value) && !(leaving && LEAVES.has(a.value)));
}

/** Uma parte da frase (L58): condição ou ação, com o tom. */
export interface Term {
  text: string;
  tone: "condition" | "action" | "danger";
  icon?: string;
}

const capitalize = (text: string) => text.charAt(0).toUpperCase() + text.slice(1);

function folderName(folders: Folder[], id: string): string {
  return folders.find((f) => f.id === id)?.name ?? "uma pasta";
}

export function conditionText(c: RuleCondition): string {
  const f = fieldInfo(c.field);
  if (f.kind === "text") return `${f.label} ${TEXT_OPS.find((o) => o.value === c.op)?.label ?? ""} ${c.value.trim()}`.trim();
  if (f.kind === "number") return `${capitalize(f.yes!)} ${c.value} ${f.unit}`;
  switch (c.field) {
    case "list": return c.op === "yes" ? "É de uma lista de e-mails" : "Não é de uma lista de e-mails";
    case "has_files": return c.op === "yes" ? "Tem arquivos anexados" : "Sem arquivos anexados";
    case "only_to_me": return c.op === "yes" ? "Enviado só para mim" : "Não foi só para mim";
    case "seen": return c.op === "yes" ? "Já lida" : "Não lida";
    case "flagged": return c.op === "yes" ? "Com estrela" : "Sem estrela";
    default: return "Não é o mais recente do remetente";
  }
}

export function actionText(a: RuleAction, folders: Folder[]): string {
  switch (a.kind) {
    case "move": return `Mover para ${folderName(folders, a.folder)}`;
    case "archive": return "Arquivar";
    case "delete": return "Excluir";
    case "copy": return `Marcador ${folderName(folders, a.folder)}`;
    case "mark_read": return "Marcar como lida";
    case "star": return "Estrela";
    case "followup": return `Sinalizar ${FOLLOWUP_DAYS.find((d) => Number(d.value) === a.days)?.label ?? `em ${a.days} dias`}`;
    case "silence": return "Não notificar";
    case "notify_always": return "Notificar sempre";
  }
}

/** A frase da regra (L58): condições, exceções e ações. */
export function sentence(spec: RuleSpec, folders: Folder[]): { conditions: Term[]; exceptions: Term[]; actions: Term[]; joiner: string } {
  return {
    conditions: spec.conditions.map((c) => ({ text: conditionText(c), tone: "condition" })),
    exceptions: spec.exceptions.map((c) => ({ text: conditionText(c), tone: "condition" })),
    actions: spec.actions.map((a) => ({
      text: actionText(a, folders),
      tone: a.kind === "delete" ? "danger" : "action",
      icon: ACTIONS.find((x) => x.value === a.kind)?.icon,
    })),
    joiner: spec.matchAll ? "e" : "ou",
  };
}

export const isCleanup = (spec: RuleSpec) => spec.conditions.some((c) => fieldInfo(c.field).cleanup);

/** Nome sugerido a partir da primeira condição (CA2). */
export function suggestedName(spec: RuleSpec): string {
  const c = spec.conditions[0];
  if (!c) return "Todos os e-mails";
  const first = c.value.split(",")[0]?.trim() ?? "";
  switch (c.field) {
    case "from":
    case "from_domain":
      return first ? `De ${first}` : "Nova regra";
    case "subject":
      return first ? `Assunto ${first}` : "Nova regra";
    case "list":
      return "Listas de e-mails";
    default:
      return conditionText(c).slice(0, 80);
  }
}

/** O que impede salvar, ou `undefined`. Regra sem condição só com confirmação (quem chama pede). */
export function saveProblem(name: string, spec: RuleSpec): string | undefined {
  if (!name.trim()) return "Dê um nome à regra.";
  if (spec.actions.length === 0) return "Escolha pelo menos uma ação.";
  if (spec.actions.some((a) => (a.kind === "move" || a.kind === "copy") && !a.folder)) return "Escolha a pasta de cada ação.";
  for (const c of [...spec.conditions, ...spec.exceptions]) {
    const kind = fieldInfo(c.field).kind;
    if (kind === "text" && !c.value.split(",").some((v) => v.trim())) return "Preencha o texto de cada condição.";
    if (kind === "number" && !(Number(c.value) > 0)) return "Use um número maior que zero.";
  }
  return undefined;
}

export function emptySpec(): RuleSpec {
  return { v: SPEC_VERSION, folders: [], matchAll: true, conditions: [conditionFor("from")], exceptions: [], actions: [actionFor("move")], stop: false };
}

/** Modelos do estado vazio e de "Nova regra" (CA7): abrem o editor preenchido. */
export const TEMPLATES: Array<{ id: string; title: string; description: string; icon: string; name: string; spec: () => RuleSpec }> = [
  {
    id: "newsletters",
    title: "Newsletters",
    description: "E-mails de listas vão para uma pasta de leitura, sem notificar.",
    icon: "mail-open",
    name: "Newsletters",
    spec: () => ({ ...emptySpec(), conditions: [{ field: "list", op: "yes", value: "" }], actions: [{ kind: "move", folder: "" }, { kind: "silence" }] }),
  },
  {
    id: "automatic",
    title: "Avisos automáticos",
    description: "De no-reply e notificações: arquivar e marcar como lidos.",
    icon: "bell-off",
    name: "Avisos automáticos",
    spec: () => ({ ...emptySpec(), conditions: [{ field: "from", op: "contains", value: "no-reply, noreply, notifications, notificacoes" }], actions: [{ kind: "archive" }, { kind: "mark_read" }] }),
  },
  {
    id: "invoices",
    title: "Recibos e faturas",
    description: "Assunto com fatura, boleto ou nota fiscal ganha estrela e prazo.",
    icon: "receipt",
    name: "Recibos e faturas",
    spec: () => ({ ...emptySpec(), conditions: [{ field: "subject", op: "contains", value: "fatura, boleto, nota fiscal, recibo" }], actions: [{ kind: "star" }, { kind: "followup", days: 7 }] }),
  },
  {
    id: "people",
    title: "Pessoas importantes",
    description: "Sempre notificar e marcar com estrela, até no Não perturbe.",
    icon: "star",
    name: "Pessoas importantes",
    spec: () => ({ ...emptySpec(), conditions: [{ field: "from", op: "is", value: "" }], actions: [{ kind: "notify_always" }, { kind: "star" }], stop: true }),
  },
];

export const emailsLabel = (n: number) => (n === 1 ? "1 e-mail" : `${n.toLocaleString("pt-BR")} e-mails`);

/** "moveu 5 e-mails para Leitura" (CA6). */
export function runText(run: RuleRun): string {
  const s = run.summary ?? {};
  const parts: string[] = [];
  if (s.moved) parts.push(`moveu ${emailsLabel(s.moved)}${s.folder ? ` para ${s.folder}` : ""}`);
  if (s.archived) parts.push(`arquivou ${emailsLabel(s.archived)}`);
  if (s.deleted) parts.push(`mandou ${emailsLabel(s.deleted)} para a Lixeira`);
  if (s.copied) parts.push(`aplicou um marcador em ${emailsLabel(s.copied)}`);
  if (s.read) parts.push(`marcou ${emailsLabel(s.read)} como lidos`);
  if (s.starred) parts.push(`deu estrela a ${emailsLabel(s.starred)}`);
  if (s.followup) parts.push(`sinalizou ${emailsLabel(s.followup)}`);
  const text = parts.length ? parts.join(", ") : `agiu em ${emailsLabel(run.count)}`;
  const origin = run.origin === "manual" ? " (rodada por você)" : run.origin === "cleanup" ? " (limpeza diária)" : "";
  return `${text}${origin}`;
}

/** "Hoje, 09:12", "Ontem, 03:00" ou "24 out". */
export function runWhen(at: number, now = new Date()): string {
  const d = new Date(at * 1000);
  const time = d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  const day = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((day(now) - day(d)) / 86_400_000);
  if (diff === 0) return `Hoje, ${time}`;
  if (diff === 1) return `Ontem, ${time}`;
  return d.toLocaleDateString("pt-BR", { day: "numeric", month: "short" }).replace(".", "");
}

/** Pastas que servem de destino e de origem: sem "Com estrela" (é a estrela, não uma pasta). */
export function targetFolders(folders: Folder[]): Folder[] {
  return folders.filter((f) => f.role !== "starred");
}

/** Pastas extras onde a regra pode rodar ao sincronizar (D2): sem Caixa de entrada, Spam, Lixeira, Enviados e Rascunhos. */
export function sourceFolders(folders: Folder[]): Folder[] {
  return folders.filter((f) => f.role === "custom" || f.role === "all");
}

/** "5 movidos, 2 arquivados" para o aviso depois da sincronização (CA6). */
export function appliedText(e: { moved: number; archived: number; deleted: number }): string | undefined {
  const parts = [
    e.moved && `${e.moved} ${e.moved === 1 ? "movido" : "movidos"}`,
    e.archived && `${e.archived} ${e.archived === 1 ? "arquivado" : "arquivados"}`,
    e.deleted && `${e.deleted} na Lixeira`,
  ].filter(Boolean);
  return parts.length ? parts.join(", ") : undefined;
}

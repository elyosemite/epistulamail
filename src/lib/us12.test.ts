import { describe, expect, it } from "vitest";
import { quickConditions } from "../features/rules/RuleDialogs";
import {
  TEMPLATES,
  appliedText,
  availableActions,
  conditionFor,
  conditionText,
  emptySpec,
  isCleanup,
  opsFor,
  runText,
  runWhen,
  saveProblem,
  sentence,
  sourceFolders,
  suggestedName,
} from "./rules";
import type { Folder, RuleRun, RuleSpec } from "./tauri";

const folders: Folder[] = [
  { id: "inbox", role: "inbox", name: "Caixa de entrada", unread: 0, total: 0 },
  { id: "leitura", role: "custom", name: "Leitura", unread: 0, total: 0 },
  { id: "all", role: "all", name: "Todos os e-mails", unread: 0, total: 0 },
  { id: "spam", role: "spam", name: "Spam", unread: 0, total: 0 },
];

describe("US12 · regras: campos e frase", () => {
  it("cada tipo de campo tem os seus operadores", () => {
    expect(opsFor("subject").map((o) => o.value)).toEqual(["contains", "not_contains", "is", "starts_with", "ends_with"]);
    expect(opsFor("list").map((o) => o.label)).toEqual(["é de uma lista", "não é de uma lista"]);
    expect(opsFor("older_than_days")).toEqual([{ value: "over", label: "mais antigo que" }]);
    expect(conditionFor("size_over_kb")).toEqual({ field: "size_over_kb", op: "over", value: "1024" });
  });

  it("a regra vira uma frase legível", () => {
    const spec: RuleSpec = {
      ...emptySpec(),
      conditions: [{ field: "list", op: "yes", value: "" }, { field: "from", op: "not_contains", value: "BBC" }],
      actions: [{ kind: "move", folder: "leitura" }, { kind: "silence" }, { kind: "delete" }],
    };
    const s = sentence(spec, folders);
    expect(s.conditions.map((t) => t.text)).toEqual(["É de uma lista de e-mails", "De não contém BBC"]);
    expect(s.actions.map((t) => t.text)).toEqual(["Mover para Leitura", "Não notificar", "Excluir"]);
    expect(s.actions[2].tone).toBe("danger");
    expect(s.joiner).toBe("e");
    expect(conditionText({ field: "older_than_days", op: "over", value: "14" })).toBe("Mais antigo que 14 dias");
  });

  it("só uma ação tira o e-mail da pasta e nenhuma se repete", () => {
    const kinds = (list: ReturnType<typeof availableActions>) => list.map((a) => a.value);
    const withMove = availableActions([{ kind: "move", folder: "x" }, { kind: "star" }], 1);
    expect(kinds(withMove)).not.toContain("archive");
    expect(kinds(withMove)).not.toContain("delete");
    expect(kinds(withMove)).not.toContain("move");
    expect(kinds(withMove)).toContain("star");
    // A própria posição pode trocar de ação de pasta.
    expect(kinds(availableActions([{ kind: "move", folder: "x" }], 0))).toContain("archive");
  });

  it("explica o que impede salvar", () => {
    const ok: RuleSpec = { ...emptySpec(), conditions: [{ field: "from", op: "is", value: "ana@x.com" }], actions: [{ kind: "move", folder: "leitura" }] };
    expect(saveProblem("Ana", ok)).toBeUndefined();
    expect(saveProblem(" ", ok)).toBe("Dê um nome à regra.");
    expect(saveProblem("Ana", { ...ok, actions: [] })).toBe("Escolha pelo menos uma ação.");
    expect(saveProblem("Ana", { ...ok, actions: [{ kind: "move", folder: "" }] })).toBe("Escolha a pasta de cada ação.");
    expect(saveProblem("Ana", { ...ok, conditions: [{ field: "subject", op: "contains", value: " , " }] })).toBe("Preencha o texto de cada condição.");
    expect(saveProblem("Ana", { ...ok, conditions: [{ field: "size_over_kb", op: "over", value: "0" }] })).toBe("Use um número maior que zero.");
  });

  it("sugere um nome e reconhece a limpeza", () => {
    expect(suggestedName({ ...emptySpec(), conditions: [{ field: "from", op: "is", value: "ana@x.com, bia@x.com" }] })).toBe("De ana@x.com");
    expect(suggestedName({ ...emptySpec(), conditions: [] })).toBe("Todos os e-mails");
    expect(isCleanup({ ...emptySpec(), conditions: [{ field: "older_than_days", op: "over", value: "30" }] })).toBe(true);
    expect(isCleanup(emptySpec())).toBe(false);
  });

  it("os modelos abrem prontos, faltando só a pasta quando há uma", () => {
    for (const t of TEMPLATES) {
      const spec = t.spec();
      const problem = saveProblem(t.name, spec);
      if (spec.actions.some((a) => a.kind === "move")) expect(problem).toBe("Escolha a pasta de cada ação.");
      else if (t.id === "people") expect(problem).toBe("Preencha o texto de cada condição.");
      else expect(problem).toBeUndefined();
    }
  });

  it("regras rodam em pastas próprias e em Todos, nunca em Spam", () => {
    expect(sourceFolders(folders).map((f) => f.id)).toEqual(["leitura", "all"]);
  });
});

describe("US12 · histórico e aviso", () => {
  const run = (summary: RuleRun["summary"], origin: RuleRun["origin"] = "sync"): RuleRun => ({ id: 1, ruleName: "N", at: 0, origin, summary, count: 5, undone: false });

  it("descreve o que cada lote fez", () => {
    expect(runText(run({ moved: 5, folder: "Leitura" }))).toBe("moveu 5 e-mails para Leitura");
    expect(runText(run({ archived: 1, read: 1 }))).toBe("arquivou 1 e-mail, marcou 1 e-mail como lidos");
    expect(runText(run({ deleted: 31 }, "cleanup"))).toBe("mandou 31 e-mails para a Lixeira (limpeza diária)");
    expect(runText(run(null, "manual"))).toBe("agiu em 5 e-mails (rodada por você)");
    expect(appliedText({ moved: 5, archived: 2, deleted: 0 })).toBe("5 movidos, 2 arquivados");
    expect(appliedText({ moved: 0, archived: 0, deleted: 0 })).toBeUndefined();
  });

  it("diz quando foi", () => {
    const now = new Date(2026, 9, 26, 10, 0);
    expect(runWhen(new Date(2026, 9, 26, 9, 12).getTime() / 1000, now)).toBe("Hoje, 09:12");
    expect(runWhen(new Date(2026, 9, 25, 3, 0).getTime() / 1000, now)).toBe("Ontem, 03:00");
    expect(runWhen(new Date(2026, 9, 24, 3, 0).getTime() / 1000, now)).toBe("24 de out");
  });

  it("criar a partir da mensagem usa as sugestões escolhidas", () => {
    const s = { fromName: "BBC", fromAddress: "news@bbc.com", domain: "bbc.com", listId: "hist.bbc.com", subject: "Vikings", folderId: "f" };
    expect(quickConditions(s, ["from", "list"])).toEqual([
      { field: "from", op: "is", value: "news@bbc.com" },
      { field: "list", op: "yes", value: "" },
    ]);
    expect(quickConditions(s, ["domain"])).toEqual([{ field: "from_domain", op: "is", value: "bbc.com" }]);
  });
});

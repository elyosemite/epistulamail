import { describe, expect, it } from "vitest";
import {
  EMPTY_SEARCH,
  addChip,
  fromQuery,
  hasCriteria,
  parseDate,
  parseOperators,
  periodLabel,
  presetChip,
  ruleFromSearch,
  searchLabel,
  toQuery,
  typingAt,
  withoutTrailingOp,
  type SearchChip,
} from "./search";

describe("US16: busca com chips", () => {
  it("lê datas digitadas e recusa as que não existem", () => {
    expect(parseDate("2026-03-31")).toBe("2026-03-31");
    expect(parseDate("31/03/2026")).toBe("2026-03-31");
    expect(parseDate("1/3/26")).toBe("2026-03-01");
    expect(parseDate("31/02/2026")).toBeNull();
    expect(parseDate("ontem")).toBeNull();
  });

  it("dá nome aos períodos", () => {
    expect(periodLabel("2026-01-01", "2026-03-31")).toBe("1 jan a 31 mar 2026");
    expect(periodLabel("2025-12-01", "2026-01-15")).toBe("1 dez 2025 a 15 jan 2026");
    expect(periodLabel("2025-01-01", "2025-12-31")).toBe("Em 2025");
    expect(periodLabel("2026-01-01", null)).toBe("Depois de 1 jan 2026");
    expect(periodLabel(null, "2026-01-01")).toBe("Antes de 1 jan 2026");
  });

  it("monta os atalhos de período a partir de hoje", () => {
    const now = new Date(2026, 9, 27);
    expect(presetChip("today", now)).toMatchObject({ after: "2026-10-27", before: "2026-10-27" });
    expect(presetChip("7d", now)).toMatchObject({ after: "2026-10-21", before: null });
    expect(presetChip("30d", now)).toMatchObject({ after: "2026-09-28" });
    expect(presetChip("year", now)).toMatchObject({ after: "2026-01-01", label: "Este ano" });
  });

  it("percebe o operador que está sendo digitado", () => {
    expect(typingAt("contrato de: yu")).toEqual({ kind: "person", role: "from", term: "yu" });
    expect(typingAt("para:")).toEqual({ kind: "person", role: "to", term: "" });
    expect(typingAt("na: arq")).toEqual({ kind: "folder", term: "arq" });
    expect(typingAt("decisão")).toBeNull();
    expect(withoutTrailingOp("contrato de: yu")).toBe("contrato");
  });

  it("transforma operadores completos em chips e deixa o resto como texto", () => {
    const r = parseOperators("contrato tem:anexo é:não-lida depois:01/01/2026 antes:2026-03-31 de: Yuri Melo");
    expect(r.text).toBe("contrato");
    expect(r.chips).toContainEqual({ kind: "attachment" });
    expect(r.chips).toContainEqual({ kind: "unread" });
    expect(r.chips).toContainEqual({ kind: "period", after: "2026-01-01", before: "2026-03-31", label: "1 jan a 31 mar 2026" });
    expect(r.chips).toContainEqual({ kind: "person", role: "from", name: "Yuri Melo", address: null });
    expect(parseOperators("cc: Ana@Exemplo.org").chips).toEqual([{ kind: "person", role: "cc", name: "", address: "ana@exemplo.org" }]);
    expect(parseOperators("em:2025 relatório").chips).toEqual([{ kind: "period", after: "2025-01-01", before: "2025-12-31", label: "Em 2025" }]);
    expect(parseOperators("depois:ontem").text).toBe("depois:ontem");
  });

  it("não repete chips e troca o período", () => {
    const yuri: SearchChip = { kind: "person", role: "from", name: "Yuri", address: "yuri@exemplo.org" };
    let chips = addChip([], yuri);
    chips = addChip(chips, { ...yuri, address: "YURI@exemplo.org" });
    chips = addChip(chips, { ...yuri, role: "to" });
    chips = addChip(chips, { kind: "attachment" });
    chips = addChip(chips, { kind: "attachment" });
    chips = addChip(chips, presetChip("7d"));
    chips = addChip(chips, presetChip("year"));
    expect(chips.map((c) => c.kind)).toEqual(["person", "person", "attachment", "period"]);
    expect(chips[3]).toMatchObject({ label: "Este ano" });
  });

  it("monta a consulta do backend", () => {
    const search = {
      ...EMPTY_SEARCH,
      text: " contrato ",
      chips: [
        { kind: "person", role: "from", name: "Yuri", address: "yuri@exemplo.org" },
        { kind: "person", role: "with", name: "Ana", address: null },
        { kind: "period", after: "2026-01-01", before: null, label: "Este ano" },
        { kind: "starred" },
        { kind: "folder", id: "f1", name: "Arquivo" },
      ] as SearchChip[],
    };
    expect(toQuery(search)).toEqual({
      text: "contrato",
      from: ["yuri@exemplo.org"],
      to: [],
      cc: [],
      with: ["Ana"],
      after: "2026-01-01",
      before: null,
      hasAttachment: false,
      unread: false,
      starred: true,
      folder: "f1",
      allAccounts: false,
      oldestFirst: false,
    });
    // A pasta é de uma conta só: em todas as contas, fica de fora.
    expect(toQuery({ ...search, allAccounts: true }).folder).toBeNull();
  });

  it("volta da consulta guardada para os chips", () => {
    const q = toQuery({ ...EMPTY_SEARCH, text: "nota", chips: [{ kind: "person", role: "to", name: "", address: "ana@exemplo.org" }, { kind: "attachment" }] });
    const back = fromQuery(q);
    expect(back.text).toBe("nota");
    expect(back.chips).toEqual([{ kind: "person", role: "to", name: "", address: "ana@exemplo.org" }, { kind: "attachment" }]);
    expect(searchLabel(back)).toBe("nota · para ana@exemplo.org · com anexo");
  });

  it("sabe quando há o que buscar", () => {
    expect(hasCriteria(EMPTY_SEARCH)).toBe(false);
    expect(hasCriteria({ ...EMPTY_SEARCH, text: "a" })).toBe(false);
    expect(hasCriteria({ ...EMPTY_SEARCH, text: "ab" })).toBe(true);
    expect(hasCriteria({ ...EMPTY_SEARCH, chips: [{ kind: "unread" }] })).toBe(true);
  });

  it("cria a regra a partir dos chips", () => {
    const spec = ruleFromSearch({
      text: "contrato",
      chips: [
        { kind: "person", role: "from", name: "Yuri", address: "yuri@exemplo.org" },
        { kind: "person", role: "from", name: "Ana", address: null },
        { kind: "person", role: "with", name: "Zé", address: null },
        { kind: "attachment" },
        { kind: "unread" },
      ],
    });
    expect(spec.matchAll).toBe(true);
    expect(spec.conditions).toEqual([
      { field: "from", op: "contains", value: "yuri@exemplo.org" },
      { field: "subject", op: "contains", value: "contrato" },
      { field: "has_files", op: "yes", value: "" },
      { field: "seen", op: "no", value: "" },
    ]);
  });
});

import { describe, expect, it } from "vitest";
import { applyAppearance, canvasToken, parseAppearance } from "./appearance";
import { flagDates, visibleBuckets } from "./flagged";

describe("sinalizadas na coluna de pastas", () => {
  it("datas de referência: fim da semana é a sexta; no fim de semana, o próprio dia", () => {
    expect(flagDates(new Date(2026, 9, 21))).toEqual({ today: "2026-10-21", weekEnd: "2026-10-23" }); // quarta
    expect(flagDates(new Date(2026, 9, 23))).toEqual({ today: "2026-10-23", weekEnd: "2026-10-23" }); // sexta
    expect(flagDates(new Date(2026, 9, 24))).toEqual({ today: "2026-10-24", weekEnd: "2026-10-24" }); // sábado
  });

  it("mostra só os prazos com mensagens, na ordem da coluna", () => {
    expect(visibleBuckets(null)).toEqual([]);
    expect(visibleBuckets({ overdue: 0, today: 0, thisWeek: 0, later: 0, noDate: 0 })).toEqual([]);
    const shown = visibleBuckets({ overdue: 2, today: 0, thisWeek: 3, later: 0, noDate: 1 });
    expect(shown.map((b) => [b.label, b.count])).toEqual([["Atrasadas", 2], ["Esta semana", 3], ["Sem data", 1]]);
  });
});

describe("aparência", () => {
  it("aceita só opções conhecidas", () => {
    expect(parseAppearance({ theme: "dark", canvas: "teal" })).toEqual({ theme: "dark", canvas: "teal" });
    expect(parseAppearance({ theme: "sepia", canvas: "url(x)" })).toEqual({ theme: "system", canvas: "" });
    expect(parseAppearance(null)).toEqual({ theme: "system", canvas: "" });
    expect(canvasToken("")).toBe("var(--color-canvas)");
    expect(canvasToken("sand")).toBe("var(--color-canvas-sand)");
  });

  it("aplica o tema e o fundo no elemento raiz", () => {
    const style = new Map<string, string>();
    const root = {
      dataset: {} as Record<string, string>,
      style: { setProperty: (k: string, v: string) => style.set(k, v), removeProperty: (k: string) => style.delete(k) },
    } as unknown as HTMLElement;
    applyAppearance({ theme: "dark", canvas: "purple" }, root);
    expect(root.dataset.theme).toBe("dark");
    expect(style.get("--app-background")).toBe("var(--color-canvas-purple)");
    applyAppearance({ theme: "system", canvas: "" }, root);
    expect(root.dataset.theme).toBeUndefined();
    expect(style.has("--app-background")).toBe(false);
  });
});

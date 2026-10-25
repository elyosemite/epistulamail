import { describe, expect, it } from "vitest";
import type { MessageHeader } from "./tauri";
import { batchTitle, commandTargets, emptySelection, masterState, prune, readActionFor, toggle, toggleAll } from "./selection";

const order = ["a", "b", "c", "d", "e"];

const msg = (id: string, seen = true): MessageHeader =>
  ({ id, fromName: null, fromAddress: null, subject: id, date: null, flags: { seen, flagged: false }, followup: null }) as unknown as MessageHeader;

describe("seleção da lista", () => {
  it("marca e desmarca uma linha", () => {
    const one = toggle(emptySelection, order, "b", false);
    expect([...one.ids]).toEqual(["b"]);
    expect(toggle(one, order, "b", false).ids.size).toBe(0);
  });

  it("Shift marca o intervalo desde a âncora, nos dois sentidos", () => {
    const start = toggle(emptySelection, order, "b", false);
    expect([...toggle(start, order, "d", true).ids].sort()).toEqual(["b", "c", "d"]);
    const back = toggle(toggle(emptySelection, order, "d", false), order, "a", true);
    expect([...back.ids].sort()).toEqual(["a", "b", "c", "d"]);
  });

  it("Shift sem âncora marca só a linha", () => {
    expect([...toggle(emptySelection, order, "c", true).ids]).toEqual(["c"]);
  });

  it("checkbox da toolbar: página inteira, estado parcial e desmarcar", () => {
    expect(masterState(emptySelection, order)).toBe("none");
    const some = toggle(emptySelection, order, "a", false);
    expect(masterState(some, order)).toBe("some");
    const all = toggleAll(some, order);
    expect(masterState(all, order)).toBe("all");
    expect(toggleAll(all, order).ids.size).toBe(0);
    expect(masterState(emptySelection, [])).toBe("none");
  });

  it("limpa os ids que saíram da lista", () => {
    const sel = toggleAll(emptySelection, order);
    const pruned = prune(sel, ["a", "c"]);
    expect([...pruned.ids].sort()).toEqual(["a", "c"]);
    expect(prune(pruned, ["a", "c"])).toBe(pruned);
  });
});

describe("alvo dos comandos", () => {
  const items = [msg("a"), msg("b", false), msg("c")];

  it("usa as marcadas na ordem da lista; sem marcadas, a aberta", () => {
    const sel = toggle(toggle(emptySelection, order, "c", false), order, "a", false);
    expect(commandTargets(sel, items, "b").map((m) => m.id)).toEqual(["a", "c"]);
    expect(commandTargets(emptySelection, items, "b").map((m) => m.id)).toEqual(["b"]);
    expect(commandTargets(emptySelection, items, null)).toEqual([]);
  });

  it("lida/não lida com seleção mista marca como lida", () => {
    expect(readActionFor(items)).toEqual({ seen: true });
    expect(readActionFor([msg("a"), msg("c")])).toEqual({ seen: false });
  });

  it("título do toast no singular e no plural", () => {
    expect(batchTitle(1, "Mensagem arquivada", (n) => `${n} arquivadas`)).toBe("Mensagem arquivada");
    expect(batchTitle(3, "Mensagem arquivada", (n) => `${n} arquivadas`)).toBe("3 conversas arquivadas");
  });
});

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dueFor, dueLabel, shortDate } from "./followup";
import { clampWidth, keyWidth, LIST_MAX, LIST_MIN, widthPercent } from "./listWidth";
import { addRecipients, parseRecipient, splitRecipients } from "./recipients";
import { UndoQueue } from "./undo";

// Sexta-feira, 23 out 2026.
const FRI23 = new Date(2026, 9, 23, 15, 0);

describe("prazos da sinalização", () => {
  it("calcula hoje, amanhã, esta semana e próxima semana", () => {
    expect(dueFor("today", FRI23)).toBe("2026-10-23");
    expect(dueFor("tomorrow", FRI23)).toBe("2026-10-24");
    expect(dueFor("this_week", FRI23)).toBe("2026-10-23");
    expect(dueFor("next_week", FRI23)).toBe("2026-10-26");
    const wed = new Date(2026, 9, 21);
    expect(dueFor("this_week", wed)).toBe("2026-10-23");
    expect(dueFor("no_date", FRI23)).toBeNull();
  });

  it("trata sexta, fim de semana e viradas de mês e de ano", () => {
    const fri = new Date(2026, 9, 30);
    expect(dueFor("this_week", fri)).toBe("2026-10-30");
    expect(dueFor("tomorrow", fri)).toBe("2026-10-31");
    expect(dueFor("next_week", fri)).toBe("2026-11-02");
    const sun = new Date(2026, 10, 1);
    expect(dueFor("this_week", sun)).toBe("2026-11-01");
    expect(dueFor("next_week", sun)).toBe("2026-11-02");
    const mon = new Date(2026, 10, 2);
    expect(dueFor("next_week", mon)).toBe("2026-11-09");
    expect(dueFor("tomorrow", new Date(2026, 11, 31))).toBe("2027-01-01");
  });

  it("mostra o prazo como lozenge", () => {
    expect(dueLabel(null, FRI23)).toBeNull();
    expect(dueLabel({ due: null, completed: false }, FRI23)).toBeNull();
    expect(dueLabel({ due: "2026-10-23", completed: false }, FRI23)).toEqual({ text: "Hoje", tone: "today" });
    expect(dueLabel({ due: "2026-10-24", completed: false }, FRI23)).toEqual({ text: "Amanhã", tone: "future" });
    expect(dueLabel({ due: "2026-10-27", completed: false }, FRI23)).toEqual({ text: "ter, 27 out", tone: "future" });
    expect(dueLabel({ due: "2026-10-20", completed: false }, FRI23)).toEqual({ text: "Atrasada", tone: "overdue" });
    expect(dueLabel({ due: "2026-10-20", completed: true }, FRI23)).toEqual({ text: "Concluída", tone: "done" });
    expect(shortDate("2026-10-24")).toBe("sáb, 24 out");
  });
});

describe("largura da lista", () => {
  it("limita, ajusta pelo teclado e anuncia em porcentagem", () => {
    expect(clampWidth(100)).toBe(LIST_MIN);
    expect(clampWidth(9999)).toBe(LIST_MAX);
    expect(keyWidth(400, "ArrowRight", false)).toBe(416);
    expect(keyWidth(400, "ArrowLeft", true)).toBe(336);
    expect(keyWidth(330, "ArrowLeft", true)).toBe(LIST_MIN);
    expect(keyWidth(400, "Home", false)).toBe(LIST_MIN);
    expect(keyWidth(400, "End", false)).toBe(LIST_MAX);
    expect(keyWidth(400, "a", false)).toBeNull();
    expect(widthPercent(LIST_MIN)).toBe(0);
    expect(widthPercent(440)).toBe(50);
    expect(widthPercent(LIST_MAX)).toBe(100);
  });
});

describe("destinatários", () => {
  it("lê nome e endereço e valida", () => {
    expect(parseRecipient("Marina Couto <marina@lumen.com.br>")).toEqual({ raw: "Marina Couto <marina@lumen.com.br>", name: "Marina Couto", address: "marina@lumen.com.br", valid: true });
    expect(parseRecipient(" ana@x.com ").valid).toBe(true);
    expect(parseRecipient("financeiro@empresa").valid).toBe(false);
    expect(parseRecipient("sem arroba").valid).toBe(false);
  });

  it("separa por vírgula, ponto e vírgula e quebra de linha, fora de aspas", () => {
    expect(splitRecipients("a@x.com, b@x.com; c@x.com\nd@x.com")).toEqual(["a@x.com", "b@x.com", "c@x.com", "d@x.com"]);
    expect(splitRecipients('"Silva, João" <j@x.com>, a@x.com')).toEqual(['"Silva, João" <j@x.com>', "a@x.com"]);
  });

  it("não repete endereços", () => {
    expect(addRecipients(["Ana <ANA@x.com>"], "ana@x.com, b@x.com, b@x.com")).toEqual(["Ana <ANA@x.com>", "b@x.com"]);
  });
});

describe("desfazer", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("chama o servidor só quando o prazo acaba", async () => {
    const q = new UndoQueue(6000);
    const run = vi.fn(() => Promise.resolve());
    const restore = vi.fn();
    q.schedule({ run, restore });
    vi.advanceTimersByTime(5999);
    expect(run).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(run).toHaveBeenCalledOnce();
    expect(restore).not.toHaveBeenCalled();
  });

  it("desfazer cancela sem chamar o servidor", () => {
    const q = new UndoQueue(6000);
    const run = vi.fn(() => Promise.resolve());
    const restore = vi.fn();
    q.schedule({ run, restore });
    expect(q.undo()).toBe(true);
    vi.advanceTimersByTime(10000);
    expect(run).not.toHaveBeenCalled();
    expect(restore).toHaveBeenCalledOnce();
    expect(q.undo()).toBe(false);
  });

  it("uma nova ação executa a anterior na hora, e a falha restaura", async () => {
    const onError = vi.fn();
    const q = new UndoQueue(6000, onError);
    const first = { run: vi.fn(() => Promise.reject(new Error("x"))), restore: vi.fn() };
    const second = { run: vi.fn(() => Promise.resolve()), restore: vi.fn() };
    q.schedule(first);
    q.schedule(second);
    expect(first.run).toHaveBeenCalledOnce();
    await vi.runAllTimersAsync();
    expect(first.restore).toHaveBeenCalledOnce();
    expect(onError).toHaveBeenCalledWith(first, expect.any(Error));
    expect(second.run).toHaveBeenCalledOnce();
  });
});

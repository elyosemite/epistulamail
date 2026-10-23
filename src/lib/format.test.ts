import { describe, expect, it } from "vitest";
import { avatarColor, formatFullDate, formatListDate, initials } from "./format";

const now = new Date(2026, 9, 23, 15, 0);

describe("formatListDate", () => {
  it("mostra só a hora para mensagens de hoje", () => {
    expect(formatListDate(new Date(2026, 9, 23, 10, 42).toISOString(), now)).toBe("10:42");
  });
  it("mostra dia e mês no ano corrente", () => {
    expect(formatListDate(new Date(2026, 9, 20, 8, 0).toISOString(), now)).toBe("20 out");
  });
  it("inclui o ano para anos anteriores", () => {
    expect(formatListDate(new Date(2025, 0, 5, 8, 0).toISOString(), now)).toBe("5 jan 2025");
  });
  it("devolve vazio para data inválida", () => {
    expect(formatListDate("não é data", now)).toBe("");
  });
});

describe("formatFullDate", () => {
  it("usa 'Hoje' no dia corrente", () => {
    expect(formatFullDate(new Date(2026, 9, 23, 9, 5).toISOString(), now)).toBe("Hoje, 09:05");
  });
  it("usa data completa nos outros dias", () => {
    expect(formatFullDate(new Date(2026, 9, 20, 18, 30).toISOString(), now)).toBe("20 out 2026, 18:30");
  });
});

describe("initials", () => {
  it("usa primeira e última palavra", () => expect(initials("Marina Couto Silva")).toBe("MS"));
  it("aceita um único nome", () => expect(initials("marina")).toBe("M"));
  it("tem fallback para vazio", () => expect(initials("")).toBe("?"));
});

describe("avatarColor", () => {
  it("é estável para o mesmo endereço", () => {
    expect(avatarColor("a@b.com")).toBe(avatarColor("a@b.com"));
  });
});

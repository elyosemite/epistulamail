import { describe, expect, it } from "vitest";
import { conversationLayout, initiallyOpen, participantsLabel, revealQuote, splitQuote } from "./conversation";
import { bellCount, bellLabel, noticeIcon, noticeText, noticeTime } from "./notices";
import type { ConversationMessage } from "./tauri";

const msg = (id: string, seen = true): ConversationMessage => ({
  id,
  fromName: "Marina",
  fromAddress: "marina@x.com",
  subject: "s",
  date: "2026-10-24T10:00:00-03:00",
  flags: { seen, flagged: false },
  followup: null,
  hasFiles: false,
  hasImages: false,
  mine: false,
});

describe("conversa em cards (D2)", () => {
  it("abre a mais recente, as não lidas e a escolhida", () => {
    const list = [msg("a"), msg("b", false), msg("c"), msg("d")];
    expect([...initiallyOpen(list, "a")].sort()).toEqual(["a", "b", "d"]);
  });

  it("recolhe as do meio de uma sequência longa", () => {
    const list = ["a", "b", "c", "d", "e", "f"].map((id) => msg(id));
    const parts = conversationLayout(list, new Set(["f"]), false);
    expect(parts.map((p) => (p.kind === "more" ? `+${p.count}` : `${p.kind[0]}:${p.message.id}`))).toEqual(["c:a", "+3", "c:e", "o:f"]);
    expect(conversationLayout(list, new Set(["f"]), true)).toHaveLength(6);
  });

  it("mantém até 4 recolhidas seguidas", () => {
    const list = ["a", "b", "c", "d", "e"].map((id) => msg(id));
    expect(conversationLayout(list, new Set(), false).map((p) => p.kind)).toEqual(["collapsed", "collapsed", "collapsed", "collapsed", "open"]);
  });

  it("lista os participantes com 'você'", () => {
    const people = (...names: Array<string | null>) => ({ count: 2, unread: false, flagged: false, participants: names.map((n) => ({ name: n ?? "", mine: n === null })) });
    expect(participantsLabel(people("Marina Couto", "Rafael Dias", "Paulo Henrique"), "x")).toBe("Marina, Rafael, Paulo");
    expect(participantsLabel(people("Paulo Henrique", null), "x")).toBe("Paulo Henrique, você");
    expect(participantsLabel(people(null), "x")).toBe("Você");
    expect(participantsLabel(people(), "Banco")).toBe("Banco");
  });
});

describe("texto citado (D3)", () => {
  it("separa a partir do cabeçalho da citação", () => {
    const text = "Fechado, mando amanhã.\r\n\r\nEm 24 de out. de 2026, às 18:10, Marina Couto escreveu:\r\n> Perfeito.\r\n";
    expect(splitQuote(text)).toEqual({ own: "Fechado, mando amanhã.", quote: "Em 24 de out. de 2026, às 18:10, Marina Couto escreveu:\n> Perfeito." });
  });

  it("separa o bloco final de linhas com >", () => {
    expect(splitQuote("Ok.\n\n> a\n>\n> b\n").quote).toBe("> a\n>\n> b");
    expect(splitQuote("> só citação\n> nada mais").quote).toBeNull();
    expect(splitQuote("Linha > no meio\nfim").quote).toBeNull();
  });

  it("mostra a citação do HTML tirando só a regra marcada", () => {
    const doc = "<style>body{margin:0}/*quote*/.gmail_quote{display:none!important}/*quote*/</style><body>/*quote*/x</body>";
    expect(revealQuote(doc)).toBe("<style>body{margin:0}</style><body>/*quote*/x</body>");
  });
});

describe("sino (CA7)", () => {
  it("conta até 9+", () => {
    expect(bellCount(0)).toBeNull();
    expect(bellCount(2)).toBe("2");
    expect(bellCount(12)).toBe("9+");
    expect(bellLabel(1, false)).toBe("Notificações, 1 não vista");
    expect(bellLabel(12, true)).toBe("Notificações, Não perturbe ligado, mais de 9 não vistas");
  });

  it("mostra a hora, 'Ontem' ou a data", () => {
    const now = new Date(2026, 9, 25, 12, 0);
    expect(noticeTime(new Date(2026, 9, 25, 9, 15).getTime() / 1000, now)).toBe("09:15");
    expect(noticeTime(new Date(2026, 9, 24, 20, 0).getTime() / 1000, now)).toBe("Ontem");
    expect(noticeTime(new Date(2026, 9, 20, 8, 0).getTime() / 1000, now)).toBe("20 out");
  });

  it("escolhe o ícone e junta as linhas", () => {
    expect(noticeIcon("failure")).toEqual({ icon: "error", tone: "danger" });
    expect(noticeIcon("reauth").tone).toBe("warning");
    expect(noticeText("Re: Contrato", "Trabalho · resposta a um e-mail seu")).toBe("Re: Contrato · Trabalho · resposta a um e-mail seu");
    expect(noticeText("Abra o Epistula para ler.", "")).toBe("Abra o Epistula para ler.");
  });
});

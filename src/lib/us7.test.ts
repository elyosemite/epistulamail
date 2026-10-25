import { describe, expect, it } from "vitest";
import { savedLabel, syncedLabel, unreadLabel } from "./accountMenu";
import { DEFAULT_APPEARANCE, parseAppearance } from "./appearance";
import { checkSend, forwardedBlock, MAX_SEND_BYTES, outgoingBody, quoteHeader, quotedLines } from "./compose";
import type { Quote } from "./tauri";

const QUOTE: Quote = {
  from: "Marina Couto <marina@lumen.com.br>",
  sent: "2026-10-23T10:42:00-03:00",
  to: "João <joao@gmail.com>",
  cc: "",
  subject: "Revisão do contrato",
  text: "Segue a versão 3.\r\n\r\n> resposta antiga\r\nAbraços,\r\nMarina\r\n",
};

describe("envio: citação e encaminhamento", () => {
  it("cita a original com cabeçalho e linhas com >", () => {
    expect(quoteHeader(QUOTE)).toMatch(/^Em 23 de out\.? de 2026, às \d\d:42, Marina Couto <marina@lumen\.com\.br> escreveu:$/);
    expect(quotedLines(QUOTE.text)).toBe("> Segue a versão 3.\n>\n>> resposta antiga\n> Abraços,\n> Marina");
    expect(quoteHeader({ ...QUOTE, sent: "" })).toBe("Marina Couto <marina@lumen.com.br> escreveu:");
  });

  it("monta o corpo de cada tipo", () => {
    const reply = outgoingBody("reply", "Ok do meu lado.\r\n", QUOTE, true);
    expect(reply.startsWith("Ok do meu lado.\n\nEm ")).toBe(true);
    expect(reply).toContain("> Segue a versão 3.");
    expect(outgoingBody("reply_all", "Ok.", QUOTE, false)).toBe("Ok.");
    expect(outgoingBody("new", "Oi.", null, true)).toBe("Oi.");
    const fwd = outgoingBody("forward", "Para seu conhecimento.", QUOTE, true);
    expect(fwd).toContain("Para seu conhecimento.\n\n---------- Mensagem encaminhada ----------\nDe: Marina Couto");
    expect(fwd).toContain("Assunto: Revisão do contrato\nPara: João <joao@gmail.com>\n\nSegue a versão 3.");
    expect(forwardedBlock({ ...QUOTE, cc: "equipe@lumen.com.br" })).toContain("Cc: equipe@lumen.com.br");
  });

  it("Enviar exige um destinatário válido e anexos até 25 MB", () => {
    const none = { to: [], cc: [], bcc: [] };
    expect(checkSend(none)).toEqual({ ok: false, reason: "Adicione pelo menos um destinatário" });
    expect(checkSend({ ...none, bcc: ["oculto@x.com"] })).toEqual({ ok: true });
    expect(checkSend({ ...none, to: ["Ana <ana@x.com>", "ana@"] })).toEqual({ ok: false, reason: "Corrija o endereço em vermelho" });
    expect(checkSend({ ...none, to: ["ana@x.com"] }, MAX_SEND_BYTES + 1)).toEqual({ ok: false, reason: "Os anexos passam do limite de 25 MB" });
  });
});

describe("menu da conta", () => {
  const now = Date.UTC(2026, 9, 25, 12, 0, 0);
  const at = (secondsAgo: number) => now / 1000 - secondsAgo;

  it("diz quando sincronizou, sem citar o provedor", () => {
    expect(syncedLabel(null, now)).toBe("Ainda não sincronizado");
    expect(syncedLabel(at(20), now)).toBe("Sincronizado agora");
    expect(syncedLabel(at(125), now)).toBe("Sincronizado há 2 min");
    expect(syncedLabel(at(3 * 3600 + 5), now)).toBe("Sincronizado há 3 h");
    expect(syncedLabel(at(3 * 86400), now)).toMatch(/^Sincronizado em \d+ de out$/);
  });

  it("conta não lidas e o que está salvo", () => {
    expect(unreadLabel(0)).toBe("Nenhuma não lida");
    expect(unreadLabel(1)).toBe("1 não lida");
    expect(unreadLabel(1200)).toBe("1.200 não lidas");
    expect(savedLabel(2430, 310 * 1024 * 1024)).toBe("2.430 mensagens salvas neste computador · 310 MB");
    expect(savedLabel(1, 820 * 1024)).toBe("1 mensagem salva neste computador · 820 KB");
  });
});

describe("preferências da lista", () => {
  it("densidade e colunas só aceitam valores conhecidos", () => {
    expect(parseAppearance({ density: "compact", showFolders: false, showReader: true })).toMatchObject({ density: "compact", showFolders: false, showReader: true });
    expect(parseAppearance({ density: "dense", showFolders: "no" })).toMatchObject({ density: "standard", showFolders: true });
    expect(DEFAULT_APPEARANCE).toMatchObject({ density: "standard", showFolders: true, showReader: true });
  });
});

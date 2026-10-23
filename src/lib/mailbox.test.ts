import { describe, expect, it } from "vitest";
import { adjustUnread, applyFlags, restoreFlags, snapshotFlags, unreadDelta } from "./flags";
import { canGoBack, currentCursor, firstPage, goBack, goForward, rangeLabel, withNext } from "./pagination";
import { attachmentKind, formatBytes, formatPeople, paragraphs } from "./format";
import type { Folder, MessageHeader } from "./tauri";

describe("pilha de cursores", () => {
  it("avança pelo cursor do backend e volta exatamente para a página anterior", () => {
    let p = firstPage();
    expect(currentCursor(p)).toBeNull();
    expect(canGoBack(p)).toBe(false);
    p = goForward(withNext(p, "c1.a"));
    expect(currentCursor(p)).toBe("c1.a");
    p = goForward(withNext(p, "c1.b"));
    expect(currentCursor(p)).toBe("c1.b");
    p = goBack(p);
    expect(currentCursor(p)).toBe("c1.a");
    p = goBack(goBack(p));
    expect(currentCursor(p)).toBeNull();
  });

  it("não avança sem próxima página", () => {
    const p = firstPage();
    expect(goForward(p)).toBe(p);
  });

  it("monta o rótulo da toolbar", () => {
    expect(rangeLabel(firstPage(), 50, 1284)).toBe("1–50 de 1.284");
    expect(rangeLabel(goForward(withNext(firstPage(), "c")), 50, 1284)).toBe("51–100 de 1.284");
    expect(rangeLabel(firstPage(), 0, 0)).toBe("0 de 0");
  });
});

const msg = (id: string, seen: boolean, flagged = false): MessageHeader => ({
  id, fromName: "", fromAddress: "", subject: "", date: "", flags: { seen, flagged },
});

describe("flags otimistas", () => {
  it("aplica e reverte", () => {
    const items = [msg("a", false), msg("b", true)];
    const before = snapshotFlags(items, ["a"]);
    const applied = applyFlags(items, ["a"], { seen: true });
    expect(applied[0].flags.seen).toBe(true);
    expect(applied[1]).toBe(items[1]);
    expect(restoreFlags(applied, before)[0].flags.seen).toBe(false);
  });

  it("calcula a variação de não lidas só para quem muda", () => {
    const before = snapshotFlags([msg("a", false), msg("b", true)], ["a", "b"]);
    expect(unreadDelta(before, { seen: true })).toBe(-1);
    expect(unreadDelta(before, { seen: false })).toBe(1);
    expect(unreadDelta(before, { flagged: true })).toBe(0);
  });

  it("ajusta o contador da pasta sem ficar negativo", () => {
    const folders: Folder[] = [{ id: "f", role: "inbox", name: "Caixa de entrada", unread: 1, total: 9 }];
    expect(adjustUnread(folders, "f", -3)[0].unread).toBe(0);
    expect(adjustUnread(folders, "f", 2)[0].unread).toBe(3);
  });
});

describe("leitura", () => {
  it("formata destinatários", () => {
    expect(formatPeople([{ name: "Ana", address: "ana@x.com" }, { name: "", address: "b@y.com" }])).toBe("Ana <ana@x.com>, b@y.com");
  });

  it("divide texto puro em parágrafos e preserva quebras simples", () => {
    expect(paragraphs("Oi\r\nTudo bem?\r\n\r\n\r\nAté")).toEqual([["Oi", "Tudo bem?"], ["Até"]]);
    expect(paragraphs("  \n ")).toEqual([]);
  });

  it("formata tamanhos e tipos de anexo", () => {
    expect(formatBytes(240 * 1024)).toBe("240 KB");
    expect(formatBytes(1.8 * 1024 * 1024)).toBe("1,8 MB");
    expect(formatBytes(512)).toBe("512 B");
    expect(attachmentKind("application/pdf")).toBe("pdf");
    expect(attachmentKind("image/png")).toBe("img");
    expect(attachmentKind("application/vnd.openxmlformats-officedocument.wordprocessingml.document")).toBe("doc");
    expect(attachmentKind("application/zip")).toBe("other");
  });
});

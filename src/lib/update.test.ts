import { describe, expect, it } from "vitest";
import { notesBlocks, plainText } from "./update";

describe("atualização: notas da release", () => {
  it("tira a marcação inline", () => {
    expect(plainText("**Instalador** sem `assinatura`: veja [o site](https://x.y).")).toBe("Instalador sem assinatura: veja o site.");
  });

  it("separa títulos, listas e parágrafos", () => {
    const md = "### Mudou\n\n- Novo ícone.\n- Nome EpistulaMail.\n\n### Avisos\n\nInstale por cima.\n";
    expect(notesBlocks(md)).toEqual([
      { kind: "heading", text: "Mudou" },
      { kind: "list", items: ["Novo ícone.", "Nome EpistulaMail."] },
      { kind: "heading", text: "Avisos" },
      { kind: "text", text: "Instale por cima." },
    ]);
  });

  it("sem notas, não há blocos", () => {
    expect(notesBlocks("")).toEqual([]);
  });
});

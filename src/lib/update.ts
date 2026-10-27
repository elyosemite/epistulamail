// Atualização automática: as notas da release (o trecho do CHANGELOG, em Markdown) em blocos simples para a
// confirmação. Só títulos, itens de lista e parágrafos; ênfase e código viram texto.

export type NotesBlock = { kind: "heading"; text: string } | { kind: "list"; items: string[] } | { kind: "text"; text: string };

/** Tira a marcação inline do Markdown (`**negrito**`, `` `código` ``, [link](url)), deixando o texto. */
export function plainText(line: string): string {
  return line
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .trim();
}

export function notesBlocks(markdown: string): NotesBlock[] {
  const blocks: NotesBlock[] = [];
  for (const raw of markdown.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    const heading = /^#{1,6}\s+(.*)$/.exec(line);
    const item = /^[-*]\s+(.*)$/.exec(line);
    if (heading) {
      blocks.push({ kind: "heading", text: plainText(heading[1]) });
    } else if (item) {
      const last = blocks[blocks.length - 1];
      if (last?.kind === "list") last.items.push(plainText(item[1]));
      else blocks.push({ kind: "list", items: [plainText(item[1])] });
    } else {
      blocks.push({ kind: "text", text: plainText(line) });
    }
  }
  return blocks;
}

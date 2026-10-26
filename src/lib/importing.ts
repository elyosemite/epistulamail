// Abrir arquivos de e-mail de outros programas (US15): textos e contas do passo 2 ao resumo.
import { formatBytes } from "./format";
import type { Account, Archive, ImportFormat, ImportScan, ImportStop, SourceFolder } from "./tauri";

/** O que o app reconheceu, como aparece no passo 2 e em Configurações. */
export const FORMAT_LABEL: Record<ImportFormat, string> = {
  takeout: "mbox do Google Takeout",
  mbox: "Arquivo mbox",
  thunderbird: "Pastas do Thunderbird",
  apple_mail: "Caixas do Apple Mail",
  maildir: "Pastas Maildir",
  eml: "Mensagens .eml",
  emlx: "Mensagem do Apple Mail (.emlx)",
  pst: "Arquivo de dados do Outlook (.pst)",
  msg: "Mensagem do Outlook (.msg)",
};

/** Ajuda do passo 1: onde cada programa deixa o backup. */
export const WHERE_TO_FIND: { title: string; icon: string; desc: string }[] = [
  { title: "Google Takeout", icon: "archive", desc: "takeout.google.com → E-mail → um arquivo .mbox com todos os e-mails e os marcadores" },
  { title: "Outlook", icon: "archive", desc: "Arquivo → Abrir e exportar → Exportar para um arquivo .pst; mensagens soltas em .msg" },
  { title: "Thunderbird", icon: "archive", desc: "A pasta do perfil, em Mail ou ImapMail (Ajuda → Informações para solução de problemas → Pasta do perfil)" },
  { title: "Apple Mail", icon: "archive", desc: "Caixa de correio → Exportar caixa de correio, um pacote .mbox" },
  { title: "Outros programas", icon: "mail", desc: "Arquivos .eml, mbox e pastas Maildir" },
];

export const ARCHIVE_KIND = "Arquivo deste computador";

export function isArchive(account: Pick<Account, "status">): boolean {
  return account.status === "local";
}

/** Chaves marcadas ao abrir o passo 2 (tudo menos Spam e Lixeira). */
export function defaultSelection(scan: Pick<ImportScan, "folders">): Set<string> {
  return new Set(scan.folders.filter((f) => f.selected).map((f) => f.key));
}

/** Pastas do passo 2: as normais uma a uma; os marcadores do Takeout numa linha só (D4). */
export function folderRows(folders: SourceFolder[]): { folders: SourceFolder[]; labels: SourceFolder[] } {
  return { folders: folders.filter((f) => !f.label), labels: folders.filter((f) => f.label) };
}

/** Quanto as pastas marcadas ocupam: no Takeout as pastas se repetem, então nunca mais que o arquivo. */
export function neededBytes(scan: Pick<ImportScan, "folders" | "bytes">, selected: Set<string>): number {
  const sum = scan.folders.filter((f) => selected.has(f.key)).reduce((n, f) => n + f.bytes, 0);
  return Math.min(sum, scan.bytes);
}

/** Mensagens que entram: no Takeout a mesma mensagem está em várias pastas, então o total do arquivo. */
export function selectedCount(scan: Pick<ImportScan, "folders" | "estimate" | "format">, selected: Set<string>): number {
  const sum = scan.folders.filter((f) => selected.has(f.key)).reduce((n, f) => n + f.count, 0);
  return scan.format === "takeout" ? Math.min(sum, scan.estimate) : sum;
}

/** Cabe no disco com a margem (D6)? `null` quando o sistema não diz o espaço livre. */
export function fitsOnDisk(scan: Pick<ImportScan, "folders" | "bytes" | "freeBytes" | "marginBytes">, selected: Set<string>): boolean | null {
  if (scan.freeBytes === null) return null;
  return neededBytes(scan, selected) + scan.marginBytes <= scan.freeBytes;
}

export function missingBytes(scan: Pick<ImportScan, "folders" | "bytes" | "freeBytes" | "marginBytes">, selected: Set<string>): number {
  if (scan.freeBytes === null) return 0;
  return Math.max(0, neededBytes(scan, selected) + scan.marginBytes - scan.freeBytes);
}

export function messagesLabel(n: number): string {
  return n === 1 ? "1 mensagem" : `${n.toLocaleString("pt-BR")} mensagens`;
}

export function diskLine(scan: Pick<ImportScan, "folders" | "bytes" | "freeBytes">, selected: Set<string>): string {
  const needed = `Vai ocupar cerca de ${formatBytes(neededBytes(scan, selected))}`;
  return scan.freeBytes === null ? needed : `${needed} · ${formatBytes(scan.freeBytes)} livres no disco`;
}

/** Quanto da barra de espaço: o que vai ocupar sobre o que está livre. */
export function diskShare(scan: Pick<ImportScan, "folders" | "bytes" | "freeBytes">, selected: Set<string>): number {
  if (!scan.freeBytes) return 0;
  return Math.min(100, Math.round((neededBytes(scan, selected) / scan.freeBytes) * 100));
}

const FAILURE_TEXT: Record<string, { title: (n: number) => string; desc: string }> = {
  too_large: {
    title: (n) => (n === 1 ? "1 grande demais" : `${n.toLocaleString("pt-BR")} grandes demais`),
    desc: "Mais de 50 MB cada; o limite protege o app de arquivos malformados.",
  },
  unreadable: {
    title: (n) => (n === 1 ? "1 ilegível" : `${n.toLocaleString("pt-BR")} ilegíveis`),
    desc: "O trecho do arquivo não forma uma mensagem.",
  },
};

/** Falhas do resumo, agrupadas por motivo (nunca o conteúdo). */
export function failureLines(failures: Record<string, number>): { title: string; desc: string }[] {
  return Object.entries(failures)
    .filter(([, n]) => n > 0)
    .map(([kind, n]) => {
      const text = FAILURE_TEXT[kind];
      return text ? { title: text.title(n), desc: text.desc } : { title: `${n} com erro`, desc: "Não foi possível ler." };
    });
}

export function failedTotal(failures: Record<string, number>): number {
  return Object.values(failures).reduce((a, b) => a + b, 0);
}

/** Toast do fim da importação, quando o assistente já não está aberto. */
export function finishedToast(archive: Pick<Archive, "imported">, name: string, stopped: ImportStop): { kind: "success" | "warning" | "error"; title: string; description: string } {
  switch (stopped) {
    case null:
    case "cancelled":
      return { kind: "success", title: `${name} está pronto`, description: `${messagesLabel(archive.imported)} neste computador.` };
    case "no_space":
      return { kind: "warning", title: `A importação de ${name} parou`, description: "O disco ficou sem espaço. Libere espaço e continue pelo aviso ou pelo menu do arquivo." };
    case "source_missing":
      return { kind: "warning", title: `A importação de ${name} parou`, description: "O arquivo de origem não está mais onde estava." };
    case "error":
      return { kind: "error", title: `A importação de ${name} parou`, description: "Algo deu errado ao gravar. O que já entrou continua." };
  }
}

/** Progresso: "12.480 de 47.130 mensagens · Caixa de entrada". */
export function progressLine(imported: number, estimate: number, folder: string | null): string {
  const counts = estimate > 0 ? `${imported.toLocaleString("pt-BR")} de ${estimate.toLocaleString("pt-BR")} mensagens` : messagesLabel(imported);
  return folder ? `${counts} · ${folder}` : counts;
}

export function progressShare(imported: number, estimate: number): number {
  return estimate > 0 ? Math.min(100, Math.round((imported / estimate) * 100)) : 0;
}

/** Sugestão de nome: o nome do arquivo sem extensão, até 32 caracteres. */
export function suggestedName(sourceName: string): string {
  const base = sourceName.replace(/\.(mbox|pst|ost|msg|eml|emlx|mbx)$/i, "").trim();
  return Array.from(base || "Arquivo").slice(0, 32).join("");
}

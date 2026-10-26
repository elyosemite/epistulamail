import { describe, expect, it } from "vitest";
import {
  defaultSelection,
  diskLine,
  failedTotal,
  failureLines,
  finishedToast,
  fitsOnDisk,
  folderRows,
  missingBytes,
  neededBytes,
  progressLine,
  progressShare,
  selectedCount,
  suggestedName,
} from "./importing";
import { railLabel, railTip, visibleAccounts } from "./accounts";
import type { Account, ImportScan, SourceFolder } from "./tauri";

const GB = 1024 ** 3;

const folder = (key: string, count: number, bytes: number, extra: Partial<SourceFolder> = {}): SourceFolder => ({
  key,
  name: key,
  role: "custom",
  depth: 0,
  count,
  bytes,
  label: false,
  selected: true,
  ...extra,
});

const takeout: ImportScan = {
  format: "takeout",
  name: "Todos os e-mails, incluindo Spam e Lixeira.mbox",
  bytes: 6 * GB,
  estimate: 48_000,
  freeBytes: 13 * GB,
  marginBytes: GB,
  folders: [
    folder("inbox", 31_000, 4 * GB, { role: "inbox" }),
    folder("all", 47_000, 6 * GB, { role: "all" }),
    folder("spam", 600, 0.1 * GB, { role: "spam", selected: false }),
    folder("label:Jurídico", 900, 0.2 * GB, { label: true }),
    folder("label:Viagens", 300, 0.1 * GB, { label: true }),
  ],
};

describe("US15: revisar antes de importar", () => {
  it("começa sem Spam e Lixeira e agrupa os marcadores do Takeout", () => {
    expect([...defaultSelection(takeout)]).toEqual(["inbox", "all", "label:Jurídico", "label:Viagens"]);
    const rows = folderRows(takeout.folders);
    expect(rows.folders.map((f) => f.key)).toEqual(["inbox", "all", "spam"]);
    expect(rows.labels).toHaveLength(2);
  });

  it("no Takeout, nem o espaço nem a contagem passam do arquivo", () => {
    const all = defaultSelection(takeout);
    expect(neededBytes(takeout, all)).toBe(6 * GB);
    expect(selectedCount(takeout, all)).toBe(48_000);
    expect(selectedCount(takeout, new Set(["label:Viagens"]))).toBe(300);
    const mbox: ImportScan = { ...takeout, format: "thunderbird", folders: [folder("Inbox", 10, 100), folder("Sent", 5, 50)] };
    expect(selectedCount(mbox, new Set(["Inbox", "Sent"]))).toBe(15);
  });

  it("confere o disco com a margem (D6)", () => {
    const all = defaultSelection(takeout);
    expect(fitsOnDisk(takeout, all)).toBe(true);
    const tight = { ...takeout, freeBytes: 6.5 * GB };
    expect(fitsOnDisk(tight, all)).toBe(false);
    expect(missingBytes(tight, all)).toBe(0.5 * GB);
    expect(fitsOnDisk({ ...takeout, freeBytes: null }, all)).toBeNull();
    expect(diskLine(takeout, all)).toBe("Vai ocupar cerca de 6 GB · 13 GB livres no disco");
  });
});

describe("US15: importando e resumo", () => {
  it("mostra o progresso com a pasta", () => {
    expect(progressLine(12_480, 47_130, "Caixa de entrada")).toBe("12.480 de 47.130 mensagens · Caixa de entrada");
    expect(progressLine(3, 0, null)).toBe("3 mensagens");
    expect(progressShare(12_480, 47_130)).toBe(26);
    expect(progressShare(5, 0)).toBe(0);
  });

  it("agrupa as falhas por motivo, sem conteúdo", () => {
    const failures = { too_large: 12, unreadable: 1, outro: 0 };
    expect(failedTotal(failures)).toBe(13);
    expect(failureLines(failures).map((f) => f.title)).toEqual(["12 grandes demais", "1 ilegível"]);
  });

  it("o aviso do fim depende de por que parou", () => {
    expect(finishedToast({ imported: 1 }, "Gmail antigo", null)).toMatchObject({ kind: "success", title: "Gmail antigo está pronto", description: "1 mensagem neste computador." });
    expect(finishedToast({ imported: 10 }, "Gmail antigo", "no_space").kind).toBe("warning");
    expect(finishedToast({ imported: 10 }, "Gmail antigo", "error").kind).toBe("error");
  });

  it("sugere o nome pelo arquivo", () => {
    expect(suggestedName("Outlook 2019.pst")).toBe("Outlook 2019");
    expect(suggestedName("Todos os e-mails, incluindo Spam e Lixeira.mbox")).toHaveLength(32);
    expect(suggestedName(".mbox")).toBe("Arquivo");
  });
});

describe("US15: o arquivo local na coluna de contas", () => {
  const archive: Account = {
    id: "11111111-2222-4333-8444-555555555555",
    provider: "archive",
    email: "11111111-2222-4333-8444-555555555555",
    displayName: "Gmail antigo",
    color: "blue",
    position: 2,
    status: "local",
    unread: 4,
    lastFolderId: null,
    listWidth: null,
    photo: null,
  };

  it("aparece na coluna e nunca mostra o id no lugar do e-mail", () => {
    expect(visibleAccounts([archive])).toHaveLength(1);
    expect(railLabel(archive)).toBe("Gmail antigo, arquivo deste computador, 4 não lidas");
    expect(railTip(archive, 1)).toBe("Gmail antigo · Arquivo deste computador (Ctrl+2)");
  });
});

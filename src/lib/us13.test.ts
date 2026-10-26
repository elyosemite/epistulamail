import { describe, expect, it } from "vitest";
import { addressOf, composeSecurity, fingerprintGroups, fingerprintShort, keyName, keyStatus, securityBadges, sendLabel, shortId } from "./crypto";
import type { CryptoSettings, MessageSecurity, PgpKey } from "./tauri";

const FP = "7A3F91C20BD455E8A1C06B2DEE41C8F03D9E2F17";

const settings: CryptoSettings = {
  pgpKey: FP,
  encryptNew: false,
  autoEncrypt: true,
  signUnencrypted: false,
  attachKey: true,
  autocrypt: true,
  encryptSubject: true,
  technology: "auto",
  searchOnline: "ask",
};

const key = (k: Partial<PgpKey>): PgpKey => ({
  fingerprint: FP,
  userIds: ["Ana Ribeiro <ana@x.com>"],
  emails: ["ana@x.com"],
  algorithm: "Ed25519 / X25519",
  version: 4,
  postQuantum: false,
  createdAt: 0,
  expiresAt: null,
  revoked: false,
  acceptance: "undecided",
  origin: "autocrypt",
  addedAt: 0,
  mine: false,
  ...k,
});

describe("US13 · chaves", () => {
  it("mostra a impressão digital em grupos", () => {
    expect(fingerprintGroups(FP)).toHaveLength(10);
    expect(fingerprintGroups(FP)[0]).toBe("7A3F");
    expect(fingerprintShort(FP)).toEqual(["7A3F", "91C2", "…", "3D9E", "2F17"]);
    expect(shortId(FP)).toBe("C8F0 3D9E 2F17");
  });

  it("dá a situação de cada chave", () => {
    expect(keyStatus(key({ mine: true })).text).toBe("Minha");
    expect(keyStatus(key({ acceptance: "verified" })).text).toBe("Verificada");
    expect(keyStatus(key({ acceptance: "unverified" })).text).toBe("Aceita");
    expect(keyStatus(key({})).text).toBe("Ainda não aceita");
    expect(keyStatus(key({ expiresAt: 10 }), 20).text).toBe("Vencida");
    expect(keyStatus(key({ revoked: true, mine: true })).text).toBe("Revogada");
    expect(keyName(key({}))).toBe("Ana Ribeiro");
    expect(keyName(key({ userIds: ["<ana@x.com>"] }))).toBe("ana@x.com");
  });
});

describe("US13 · leitura", () => {
  const sec = (s: Partial<MessageSecurity>): MessageSecurity => ({
    technology: "openpgp", encrypted: true, decrypted: true, postQuantum: false, signature: { state: "unsigned" }, signerAcceptance: null, ...s,
  });

  it("verde só com assinatura boa e chave conferida", () => {
    const ok = securityBadges(sec({ signature: { state: "good", fingerprint: FP }, signerAcceptance: "verified" }));
    expect(ok.map((b) => [b.text, b.tone])).toEqual([["Criptografada", "success"], ["Assinatura verificada", "success"]]);
    const unaccepted = securityBadges(sec({ signature: { state: "good", fingerprint: FP }, signerAcceptance: "undecided" }));
    expect(unaccepted.map((b) => b.tone)).toEqual(["info", "info"]);
  });

  it("avisa sem chave e com assinatura inválida", () => {
    expect(securityBadges(sec({ decrypted: false }))[0]).toMatchObject({ text: "Não foi possível abrir", tone: "warning" });
    expect(securityBadges(sec({ encrypted: false, signature: { state: "bad" } }))).toEqual([{ text: "Assinatura inválida", tone: "danger", icon: "error" }]);
    expect(securityBadges(sec({ postQuantum: true }))[0].text).toBe("Criptografada · pós-quântica");
  });
});

describe("US13 · composer", () => {
  const status = [{ address: "ana@x.com", hasKey: true, postQuantum: false }, { address: "rafael@x.com", hasKey: false, postQuantum: false }];

  it("liga sozinho quando todos têm chave", () => {
    expect(composeSecurity(settings, true, ["Ana <ana@x.com>"], status, {}, false)).toEqual({ encrypt: true, sign: true, missing: [] });
    expect(composeSecurity(settings, true, ["ana@x.com", "rafael@x.com"], status, {}, false).encrypt).toBe(false);
    expect(composeSecurity({ ...settings, autoEncrypt: false }, true, ["ana@x.com"], status, {}, false).encrypt).toBe(false);
  });

  it("resposta a mensagem criptografada liga e mostra quem não tem chave", () => {
    expect(composeSecurity(settings, true, ["rafael@x.com"], status, {}, true)).toEqual({ encrypt: true, sign: true, missing: ["rafael@x.com"] });
    expect(composeSecurity(settings, true, ["rafael@x.com"], status, { encrypt: false }, true).encrypt).toBe(false);
  });

  it("sem chave minha, nada liga; assinar segue a conta", () => {
    expect(composeSecurity(settings, false, ["ana@x.com"], status, {}, true)).toEqual({ encrypt: false, sign: false, missing: [] });
    expect(composeSecurity({ ...settings, signUnencrypted: true }, true, ["rafael@x.com"], status, {}, false)).toEqual({ encrypt: false, sign: true, missing: [] });
  });

  it("o botão diz o que vai acontecer", () => {
    expect(sendLabel({ encrypt: true, sign: true })).toBe("Enviar criptografado");
    expect(sendLabel({ encrypt: false, sign: true })).toBe("Enviar assinado");
    expect(sendLabel({ encrypt: false, sign: false })).toBe("Enviar");
    expect(addressOf("Ana <Ana@X.com>")).toBe("ana@x.com");
  });
});

describe("arquivos de chave", () => {
  it("nome de arquivo sem acento nem espaço", async () => {
    const { keyFileName } = await import("./files");
    expect(keyFileName("José da Silva", "publica")).toBe("Jose-da-Silva-publica.asc");
    expect(keyFileName("  ", "backup-secreto")).toBe("chave-backup-secreto.asc");
  });

  it("arquivo em armadura vai como texto; binario em base64", async () => {
    const { readKeyFile } = await import("./files");
    const armored = new File(["-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nabc\n"], "a.asc");
    expect(await readKeyFile(armored)).toEqual({ data: "-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nabc\n", binary: false });
    const binary = new File([new Uint8Array([0x99, 0x01, 0x0d])], "a.gpg");
    expect(await readKeyFile(binary)).toEqual({ data: "mQEN", binary: true });
  });
});

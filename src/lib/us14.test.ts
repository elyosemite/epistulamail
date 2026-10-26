import { describe, expect, it } from "vitest";
import { certStatus, certUsage, composeSecurity, pickTechnology, securityBadges } from "./crypto";
import type { CryptoSettings, MessageSecurity, RecipientKey, SmimeCert } from "./tauri";

const settings: CryptoSettings = {
  pgpKey: "7A3F91C20BD455E8A1C06B2DEE41C8F03D9E2F17",
  smimeCert: "4E1A90C7".repeat(8),
  encryptNew: false,
  autoEncrypt: true,
  signUnencrypted: false,
  attachKey: true,
  autocrypt: true,
  encryptSubject: true,
  technology: "auto",
  searchOnline: "ask",
};

const r = (address: string, hasKey: boolean, hasCertificate: boolean): RecipientKey => ({ address, hasKey, hasCertificate, postQuantum: false });

const cert = (c: Partial<SmimeCert>): SmimeCert => ({
  fingerprint: "4E1A90C7".repeat(8),
  subject: "Ana Ribeiro",
  organization: null,
  emails: ["ana@x.com"],
  issuer: "Actalis",
  serial: "01",
  algorithm: "RSA 3072",
  notBefore: 0,
  notAfter: 100,
  canSign: true,
  canEncrypt: true,
  selfSigned: false,
  origin: "message",
  addedAt: 0,
  mine: false,
  trust: "trusted",
  chain: ["Actalis"],
  ...c,
});

describe("US14 · tecnologia do envio", () => {
  it("usa a única que a conta tem", () => {
    expect(pickTechnology({ ...settings, pgpKey: null }, ["ana@x.com"], [])).toBe("smime");
    expect(pickTechnology({ ...settings, smimeCert: null }, ["ana@x.com"], [])).toBe("openpgp");
    expect(pickTechnology({ ...settings, pgpKey: null, smimeCert: null }, ["ana@x.com"], [])).toBeNull();
  });

  it("no automático, a que todos os destinatários têm; OpenPGP no empate", () => {
    const status = [r("ana@x.com", false, true), r("paulo@x.com", true, true)];
    expect(pickTechnology(settings, ["ana@x.com", "Paulo <paulo@x.com>"], status)).toBe("smime");
    expect(pickTechnology(settings, ["paulo@x.com"], status)).toBe("openpgp");
    expect(pickTechnology({ ...settings, technology: "smime" }, ["paulo@x.com"], status)).toBe("smime");
    expect(pickTechnology(settings, ["paulo@x.com"], status, "smime")).toBe("smime");
  });

  it("o cadeado olha o certificado com S/MIME", () => {
    const status = [r("ana@x.com", false, true)];
    expect(composeSecurity(settings, true, ["ana@x.com"], status, {}, false, "smime").encrypt).toBe(true);
    expect(composeSecurity(settings, true, ["ana@x.com"], status, {}, false, "openpgp").encrypt).toBe(false);
    expect(composeSecurity(settings, true, ["ana@x.com"], status, { encrypt: true }, false, "openpgp").missing).toEqual(["ana@x.com"]);
  });
});

describe("US14 · leitura", () => {
  const sec = (s: Partial<MessageSecurity>): MessageSecurity => ({
    technology: "smime",
    encrypted: false,
    decrypted: false,
    postQuantum: false,
    signature: { state: "good", fingerprint: "AB" },
    signerAcceptance: null,
    certificate: { subject: "Ana", issuer: "Actalis", trust: "trusted", emailMatches: true, notAfter: 100 },
    ...s,
  });

  it("verde só com cadeia confiável e o e-mail do remetente", () => {
    expect(securityBadges(sec({}))).toEqual([{ text: "Assinatura verificada", tone: "success", icon: "badge-check" }]);
    expect(securityBadges(sec({ certificate: { subject: "Ana", issuer: "X", trust: "untrusted", emailMatches: true, notAfter: 1 } }))[0].tone).toBe("warning");
    expect(securityBadges(sec({ certificate: { subject: "Ana", issuer: "X", trust: "expired", emailMatches: true, notAfter: 1 } }))[0].text).toBe("Assinada · certificado vencido");
    expect(securityBadges(sec({ certificate: { subject: "Ana", issuer: "X", trust: "trusted", emailMatches: false, notAfter: 1 } }))[0].text).toBe("Assinada · certificado de outro e-mail");
    expect(securityBadges(sec({ encrypted: true, decrypted: true }))[0]).toMatchObject({ text: "Criptografada", tone: "success" });
    expect(securityBadges(sec({ signature: { state: "unknown_key", issuer: null } }))[0].text).toBe("Assinada · certificado desconhecido");
  });
});

describe("US14 · certificados", () => {
  it("dá a situação e o uso", () => {
    expect(certStatus(cert({ mine: true }), 50).text).toBe("Meu");
    expect(certStatus(cert({}), 50).text).toBe("Confiável");
    expect(certStatus(cert({ trust: "untrusted" }), 50).text).toBe("Emissor não confiável");
    expect(certStatus(cert({}), 200).text).toBe("Vencido");
    expect(certUsage(cert({}))).toBe("Assinar e criptografar");
    expect(certUsage(cert({ canEncrypt: false }))).toBe("Só assinar");
  });
});

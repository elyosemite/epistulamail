// Criptografia de ponta a ponta (US13, US14): selos de leitura, rótulos do chaveiro e dos certificados, impressão digital e
// o cadeado do composer.
import type { CryptoSettings, CryptoTechnology, MessageSecurity, PgpAcceptance, PgpKey, PgpKeyKind, RecipientKey, SmimeCert, SmimeCertKind } from "./tauri";

/** "7A3F 91C2 0BD4 …": a impressão digital em grupos de 4. */
export function fingerprintGroups(fingerprint: string): string[] {
  return fingerprint.toUpperCase().match(/.{1,4}/g) ?? [];
}

/** Os primeiros e os últimos grupos, para listas (`7A3F 91C2 … 3D9E 2F17`). */
export function fingerprintShort(fingerprint: string): string[] {
  const groups = fingerprintGroups(fingerprint);
  return groups.length <= 6 ? groups : [...groups.slice(0, 2), "…", ...groups.slice(-2)];
}

/** ID curto: os últimos 12 dígitos, em grupos de 4 (como no gerenciador). */
export function shortId(fingerprint: string): string {
  return fingerprintGroups(fingerprint.slice(-12)).join(" ");
}

export const KEY_KINDS: Array<{ value: PgpKeyKind; title: string; description: string; icon: string; badge?: { text: string; tone: "success" | "new" } }> = [
  { value: "curve25519", title: "Curve25519", description: "Rápida, segura e lida por quase todos os programas.", icon: "key", badge: { text: "Recomendada", tone: "success" } },
  { value: "rsa4096", title: "RSA 4096", description: "Para quem usa programas muito antigos.", icon: "key" },
  {
    value: "post_quantum",
    title: "Pós-quântica",
    description: "ML-KEM-768 + X25519 e ML-DSA-65 + Ed25519. Protege contra computadores quânticos, mas quase nenhum outro programa lê ainda.",
    icon: "shield",
    badge: { text: "Experimental", tone: "new" },
  },
];

export const ACCEPTANCE_CHOICES: Array<{ value: PgpAcceptance; label: string }> = [
  { value: "undecided", label: "Ainda não" },
  { value: "unverified", label: "Aceitar, sem conferir" },
  { value: "verified", label: "Aceitar: conferi a impressão digital" },
  { value: "rejected", label: "Rejeitar" },
];

/** Situação de uma chave no gerenciador (lozenge). */
export function keyStatus(key: PgpKey, now = Date.now() / 1000): { text: string; tone: "success" | "inprogress" | "default" | "removed" } {
  if (key.revoked) return { text: "Revogada", tone: "removed" };
  if (key.expiresAt !== null && key.expiresAt <= now) return { text: "Vencida", tone: "removed" };
  if (key.mine) return { text: "Minha", tone: "success" };
  switch (key.acceptance) {
    case "verified": return { text: "Verificada", tone: "success" };
    case "unverified": return { text: "Aceita", tone: "inprogress" };
    case "rejected": return { text: "Rejeitada", tone: "removed" };
    default: return { text: "Ainda não aceita", tone: "default" };
  }
}

/** Nome para mostrar de uma chave: o nome da primeira identidade, ou o e-mail. */
export function keyName(key: PgpKey): string {
  const uid = key.userIds[0] ?? "";
  const name = uid.replace(/<[^>]*>/, "").trim();
  return name || key.emails[0] || shortId(key.fingerprint);
}

/** Um selo da leitura (L67). */
export interface Badge {
  text: string;
  tone: "success" | "info" | "warning" | "danger";
  icon: string;
}

/** Selo da assinatura boa: OpenPGP pela aceitação da chave; S/MIME pela cadeia e pelo e-mail do certificado (US14). */
function goodSignature(s: MessageSecurity): Badge {
  if (s.technology === "smime") {
    const c = s.certificate;
    if (!c || c.trust === "untrusted") return { text: "Assinada · emissor não confiável", tone: "warning", icon: "signature" };
    if (c.trust === "expired") return { text: "Assinada · certificado vencido", tone: "warning", icon: "signature" };
    if (!c.emailMatches) return { text: "Assinada · certificado de outro e-mail", tone: "warning", icon: "signature" };
    return { text: "Assinatura verificada", tone: "success", icon: "badge-check" };
  }
  return s.signerAcceptance === "verified"
    ? { text: "Assinatura verificada", tone: "success", icon: "badge-check" }
    : { text: s.signerAcceptance === "unverified" ? "Assinada · chave aceita sem conferir" : "Assinada · chave ainda não aceita", tone: "info", icon: "signature" };
}

/** Selos de uma mensagem aberta (CA7). Verde só para assinatura boa com chave conferida ou certificado confiável. */
export function securityBadges(s: MessageSecurity): Badge[] {
  const out: Badge[] = [];
  const signature = s.signature.state === "good" ? goodSignature(s) : null;
  if (s.encrypted) {
    out.push(
      s.decrypted
        ? { text: s.postQuantum ? "Criptografada · pós-quântica" : "Criptografada", tone: signature?.tone === "success" ? "success" : "info", icon: "lock" }
        : { text: "Não foi possível abrir", tone: "warning", icon: "lock" },
    );
  }
  switch (s.signature.state) {
    case "good":
      out.push(signature!);
      break;
    case "unknown_key":
      out.push({ text: s.technology === "smime" ? "Assinada · certificado desconhecido" : "Assinada · chave desconhecida", tone: "info", icon: "signature" });
      break;
    case "bad":
      out.push({ text: "Assinatura inválida", tone: "danger", icon: "error" });
      break;
  }
  return out;
}

/** Endereço em minúsculas de "Nome <email>" ou de um e-mail solto. */
export function addressOf(raw: string): string {
  const angle = raw.match(/<([^<>]+)>/);
  return (angle ? angle[1] : raw).trim().toLowerCase();
}

/** Estado do cadeado no composer (CA6). */
export interface ComposeSecurity {
  encrypt: boolean;
  sign: boolean;
  /** Destinatários sem chave aceita, com o cadeado ligado. */
  missing: string[];
}

/** Com S/MIME, vale o certificado; com OpenPGP, a chave. */
export const hasCredential = (s: RecipientKey, technology: CryptoTechnology) => (technology === "smime" ? s.hasCertificate : s.hasKey);

/**
 * Tecnologia do envio (US14, CA3): a que a conta tem; com as duas, a preferida, e no "automático" a que todos os destinatários
 * têm (OpenPGP no empate). A escolha do usuário no composer vale sobre tudo, desde que a conta tenha a credencial.
 */
export function pickTechnology(
  settings: CryptoSettings | null,
  recipients: string[],
  status: RecipientKey[],
  choice?: CryptoTechnology,
): CryptoTechnology | null {
  const pgp = !!settings?.pgpKey;
  const smime = !!settings?.smimeCert;
  if (!settings || (!pgp && !smime)) return null;
  if (!pgp || !smime) return pgp ? "openpgp" : "smime";
  if (choice) return choice;
  if (settings.technology !== "auto") return settings.technology;
  const addresses = recipients.map(addressOf).filter(Boolean);
  const all = (t: CryptoTechnology) => addresses.length > 0 && addresses.every((a) => status.some((s) => s.address === a && hasCredential(s, t)));
  return !all("openpgp") && all("smime") ? "smime" : "openpgp";
}

/**
 * O cadeado começa como a conta manda, liga sozinho quando todos têm chave (se a conta pedir) e sempre liga na resposta a uma
 * mensagem criptografada. A escolha do usuário (`choice`) vale sobre tudo isso. Sem chave minha, nada liga.
 */
export function composeSecurity(
  settings: CryptoSettings | null,
  hasOwnKey: boolean,
  recipients: string[],
  status: RecipientKey[],
  choice: { encrypt?: boolean; sign?: boolean },
  replyingToEncrypted: boolean,
  technology: CryptoTechnology = "openpgp",
): ComposeSecurity {
  if (!settings || !hasOwnKey) return { encrypt: false, sign: false, missing: [] };
  const known = new Map(status.map((s) => [s.address, hasCredential(s, technology)]));
  const addresses = recipients.map(addressOf).filter(Boolean);
  const allHaveKeys = addresses.length > 0 && addresses.every((a) => known.get(a) === true);
  const automatic = settings.encryptNew || replyingToEncrypted || (settings.autoEncrypt && allHaveKeys);
  const encrypt = choice.encrypt ?? automatic;
  const sign = encrypt || (choice.sign ?? settings.signUnencrypted);
  const missing = encrypt ? addresses.filter((a) => known.get(a) !== true) : [];
  return { encrypt, sign, missing };
}

/** Rótulo do botão Enviar, que repete o que vai acontecer. */
export function sendLabel(s: { encrypt: boolean; sign: boolean }): string {
  if (s.encrypt) return "Enviar criptografado";
  if (s.sign) return "Enviar assinado";
  return "Enviar";
}

/** Data curta de um carimbo em segundos ("26 out 2029"), ou "Sem validade". */
export function keyDate(seconds: number | null): string {
  if (seconds === null) return "Sem validade";
  return new Date(seconds * 1000).toLocaleDateString("pt-BR", { day: "numeric", month: "short", year: "numeric" }).replace(".", "").replace(/ de /g, " ");
}

// ---------------------------------------------------------------- S/MIME (US14)

export const CERT_KINDS: Array<{ value: SmimeCertKind; label: string }> = [
  { value: "rsa3072", label: "RSA 3072 (recomendado, aceito por todas as autoridades)" },
  { value: "p256", label: "ECDSA P-256" },
];

export const COUNTRIES = [
  { value: "BR", label: "Brasil (BR)" },
  { value: "PT", label: "Portugal (PT)" },
  { value: "AO", label: "Angola (AO)" },
  { value: "MZ", label: "Moçambique (MZ)" },
  { value: "US", label: "Estados Unidos (US)" },
  { value: "", label: "Não informar" },
];

/** Situação de um certificado no gerenciador (lozenge). */
export function certStatus(cert: SmimeCert, now = Date.now() / 1000): { text: string; tone: "success" | "inprogress" | "default" | "removed" } {
  if (cert.notAfter <= now) return { text: "Vencido", tone: "removed" };
  if (cert.notBefore > now) return { text: "Ainda não vale", tone: "default" };
  if (cert.trust !== "trusted") return { text: "Emissor não confiável", tone: "default" };
  return cert.mine ? { text: "Meu", tone: "success" } : { text: "Confiável", tone: "success" };
}

export function certUsage(cert: SmimeCert): string {
  if (cert.canSign && cert.canEncrypt) return "Assinar e criptografar";
  if (cert.canSign) return "Só assinar";
  if (cert.canEncrypt) return "Só criptografar";
  return "Sem uso para e-mail";
}

export const TRUST_LABEL: Record<SmimeCert["trust"], string> = {
  trusted: "Confiável (raiz no Windows)",
  expired: "Vencida",
  untrusted: "Não confiável: a raiz não está no Windows",
};

export const CERT_ORIGIN: Record<SmimeCert["origin"], string> = {
  request: "Pedido feito neste computador",
  imported: "Importado de um arquivo",
  pkcs12: "Importado com a chave (.p12)",
  message: "Assinatura de uma mensagem",
};

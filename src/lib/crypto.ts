// Criptografia de ponta a ponta (US13): selos de leitura, rótulos do chaveiro, impressão digital e o cadeado do composer.
import type { CryptoSettings, MessageSecurity, PgpAcceptance, PgpKey, PgpKeyKind, RecipientKey } from "./tauri";

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

/** Selos de uma mensagem aberta (CA7). Verde só para assinatura boa com chave conferida. */
export function securityBadges(s: MessageSecurity): Badge[] {
  const out: Badge[] = [];
  if (s.encrypted) {
    out.push(
      s.decrypted
        ? { text: s.postQuantum ? "Criptografada · pós-quântica" : "Criptografada", tone: s.signature.state === "good" && s.signerAcceptance === "verified" ? "success" : "info", icon: "lock" }
        : { text: "Não foi possível abrir", tone: "warning", icon: "lock" },
    );
  }
  switch (s.signature.state) {
    case "good":
      out.push(
        s.signerAcceptance === "verified"
          ? { text: "Assinatura verificada", tone: "success", icon: "badge-check" }
          : { text: s.signerAcceptance === "unverified" ? "Assinada · chave aceita sem conferir" : "Assinada · chave ainda não aceita", tone: "info", icon: "signature" },
      );
      break;
    case "unknown_key":
      out.push({ text: "Assinada · chave desconhecida", tone: "info", icon: "signature" });
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
): ComposeSecurity {
  if (!settings || !hasOwnKey) return { encrypt: false, sign: false, missing: [] };
  const known = new Map(status.map((s) => [s.address, s.hasKey]));
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

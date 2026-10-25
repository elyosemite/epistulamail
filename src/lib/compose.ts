// Texto enviado nas respostas e encaminhamentos e a validação do botão Enviar (US7, CA1–CA3).
import { parseRecipient } from "./recipients";
import type { DraftKind, Quote } from "./tauri";

/** Limite do servidor para a mensagem com anexos (25 MB). */
export const MAX_SEND_BYTES = 25 * 1024 * 1024;

/** "23 de out. de 2026, às 10:42" (o fuso é o deste computador). */
export function quoteDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const day = date.toLocaleDateString("pt-BR", { day: "numeric", month: "short", year: "numeric" });
  const time = date.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  return `${day}, às ${time}`;
}

/** Cabeçalho da citação: "Em 23 de out. de 2026, às 10:42, Marina <m@x> escreveu:". */
export function quoteHeader(quote: Quote): string {
  const when = quoteDate(quote.sent);
  return when ? `Em ${when}, ${quote.from} escreveu:` : `${quote.from} escreveu:`;
}

/** Texto citado com `> ` no começo de cada linha (D5). */
export function quotedLines(text: string): string {
  return text
    .replace(/\r\n?/g, "\n")
    .trimEnd()
    .split("\n")
    .map((line) => (line.startsWith(">") ? `>${line}` : `> ${line}`).trimEnd())
    .join("\n");
}

/** Bloco da mensagem encaminhada, abaixo do texto de quem encaminha. */
export function forwardedBlock(quote: Quote): string {
  const lines = [
    "---------- Mensagem encaminhada ----------",
    `De: ${quote.from}`,
    `Data: ${quoteDate(quote.sent) || quote.sent}`,
    `Assunto: ${quote.subject}`,
    `Para: ${quote.to}`,
    ...(quote.cc ? [`Cc: ${quote.cc}`] : []),
  ];
  return `${lines.join("\n")}\n\n${quote.text.replace(/\r\n?/g, "\n").trimEnd()}`;
}

/** Corpo final: o texto escrito e, conforme o tipo, a citação (se não foi removida) ou a encaminhada. */
export function outgoingBody(kind: DraftKind, body: string, quote: Quote | null, keepQuote: boolean): string {
  const text = body.replace(/\r\n?/g, "\n").trimEnd();
  if (!quote) return text;
  if (kind === "forward") return `${text}\n\n${forwardedBlock(quote)}`;
  if ((kind === "reply" || kind === "reply_all") && keepQuote) return `${text}\n\n${quoteHeader(quote)}\n${quotedLines(quote.text)}`;
  return text;
}

export type SendCheck = { ok: true } | { ok: false; reason: string };

/** Enviar fica ativo com pelo menos um destinatário e nenhum inválido; anexos até 25 MB. */
export function checkSend(recipients: { to: string[]; cc: string[]; bcc: string[] }, attachmentBytes = 0): SendCheck {
  const all = [...recipients.to, ...recipients.cc, ...recipients.bcc];
  if (all.length === 0) return { ok: false, reason: "Adicione pelo menos um destinatário" };
  if (all.some((r) => !parseRecipient(r).valid)) return { ok: false, reason: "Corrija o endereço em vermelho" };
  if (attachmentBytes > MAX_SEND_BYTES) return { ok: false, reason: "Os anexos passam do limite de 25 MB" };
  return { ok: true };
}

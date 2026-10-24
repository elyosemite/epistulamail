/** Destinatários digitados nos cards de resposta (US4, CA8). */

export interface Recipient {
  /** Como foi digitado ou veio do modelo: "Nome <endereco>" ou só o endereço. */
  raw: string;
  name: string;
  address: string;
  valid: boolean;
}

const EMAIL = /^[^\s@<>(),;:"]+@[^\s@<>(),;:"]+\.[^\s@<>(),;:"]+$/;

export function parseRecipient(raw: string): Recipient {
  const text = raw.trim();
  const angle = text.match(/^(.*)<([^<>]+)>$/);
  const name = angle ? angle[1].trim().replace(/^"(.*)"$/, "$1") : "";
  const address = (angle ? angle[2] : text).trim();
  return { raw: text, name, address, valid: EMAIL.test(address) };
}

/** Separa o que foi digitado em `,`, `;` ou quebra de linha (fora de aspas). */
export function splitRecipients(input: string): string[] {
  const out: string[] = [];
  let current = "";
  let quoted = false;
  for (const ch of input) {
    if (ch === '"') quoted = !quoted;
    if (!quoted && (ch === "," || ch === ";" || ch === "\n")) {
      if (current.trim()) out.push(current.trim());
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.trim()) out.push(current.trim());
  return out;
}

/** Junta novos destinatários sem repetir endereços já presentes (sem diferenciar maiúsculas). */
export function addRecipients(current: string[], input: string): string[] {
  const seen = new Set(current.map((r) => parseRecipient(r).address.toLowerCase()));
  const next = [...current];
  for (const raw of splitRecipients(input)) {
    const key = parseRecipient(raw).address.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    next.push(raw);
  }
  return next;
}

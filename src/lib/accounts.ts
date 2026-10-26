import type { Account, AccountColor } from "./tauri";

export const ACCOUNT_COLORS: { value: AccountColor; label: string }[] = [
  { value: "blue", label: "Azul" },
  { value: "teal", label: "Verde-azulado" },
  { value: "green", label: "Verde" },
  { value: "purple", label: "Roxo" },
  { value: "magenta", label: "Magenta" },
  { value: "orange", label: "Laranja" },
];

export const NAME_SUGGESTIONS = ["Pessoal", "Trabalho", "Família"];
export const MAX_ACCOUNT_NAME = 32;

/** Inicial do nome dado à conta (não do e-mail, que pode ter o mesmo domínio em todas). */
export function accountInitial(displayName: string): string {
  const first = Array.from(displayName.trim())[0];
  return first ? first.toLocaleUpperCase("pt-BR") : "?";
}

/** Classe de cor do `.avatar` (azul é a cor padrão, sem modificador). */
export function avatarColorClass(color: AccountColor): string | null {
  return color === "blue" ? null : `avatar--${color}`;
}

export function unreadBadge(unread: number | null): string | null {
  if (!unread || unread <= 0) return null;
  return unread > 99 ? "99+" : String(unread);
}

/** Nome acessível do item da coluna: "Trabalho, joao@empresa.com, 12 não lidas". */
export function railLabel(account: Account): string {
  const parts = [account.displayName, account.status === "local" ? "arquivo deste computador" : account.email];
  if (account.status === "signed_out") parts.push("você saiu desta conta");
  else if (account.status === "needs_reauth") parts.push("precisa reconectar");
  else if (account.unread && account.unread > 0) parts.push(account.unread > 99 ? "mais de 99 não lidas" : `${account.unread} não lidas`);
  return parts.join(", ");
}

export function railTip(account: Account, index: number): string {
  const shortcut = index < 9 ? ` (Ctrl+${index + 1})` : "";
  return `${account.displayName} · ${account.status === "local" ? "Arquivo deste computador" : account.email}${shortcut}`;
}

/** `Ctrl+1` a `Ctrl+9` trocam de conta pela ordem da coluna. Devolve o índice ou `null`. */
export function accountShortcut(event: Pick<KeyboardEvent, "ctrlKey" | "metaKey" | "altKey" | "shiftKey" | "key">): number | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return null;
  const n = Number(event.key);
  return Number.isInteger(n) && n >= 1 && n <= 9 ? n - 1 : null;
}

/** Contas na coluna: a de que o usuário saiu some (25 out 2026); os dados ficam e voltam ao entrar de novo. */
export function visibleAccounts(accounts: Account[]): Account[] {
  return accounts.filter((a) => a.status !== "signed_out");
}

/** Conta a abrir depois de carregar a lista: a atual se ainda existir, senão a primeira. */
export function pickActive(accounts: Account[], current: string | null): string | null {
  if (current && accounts.some((a) => a.id === current)) return current;
  return accounts[0]?.id ?? null;
}

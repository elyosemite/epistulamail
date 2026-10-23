import { invoke } from "@tauri-apps/api/core";

export type Provider = "google" | "microsoft";

export interface Account {
  provider: Provider;
  email: string;
}

export interface MessageHeader {
  uid: number;
  fromName: string;
  fromAddress: string;
  subject: string;
  date: string;
  flags: { seen: boolean; flagged: boolean };
}

export type AppErrorKind =
  | "AuthCancelled"
  | "AuthTimeout"
  | "AuthFailed"
  | "TokenExpired"
  | "ImapConnection"
  | "ImapAuth"
  | "Keyring"
  | "Config";

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

function toAppError(error: unknown): AppError {
  if (error && typeof error === "object" && "kind" in error && "message" in error) {
    return error as AppError;
  }
  return { kind: "AuthFailed", message: "Algo deu errado. Tente de novo." };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

export const api = {
  getAccount: () => call<Account | null>("get_account"),
  connectAccount: (provider: Provider) => call<Account>("connect_account", { provider }),
  cancelConnect: () => call<void>("cancel_connect"),
  listInbox: () => call<MessageHeader[]>("list_inbox"),
  disconnect: () => call<void>("disconnect"),
};

export function isAppError(error: unknown): error is AppError {
  return !!error && typeof error === "object" && "kind" in error;
}

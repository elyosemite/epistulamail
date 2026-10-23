import { invoke } from "@tauri-apps/api/core";

export type Provider = "google" | "microsoft";

export interface Account {
  provider: Provider;
  email: string;
}

export type FolderRole = "inbox" | "starred" | "sent" | "drafts" | "all" | "spam" | "trash" | "custom";

export interface Folder {
  id: string;
  role: FolderRole;
  name: string;
  unread: number | null;
  total: number | null;
}

export interface MessageFlags {
  seen: boolean;
  flagged: boolean;
}

export interface MessageHeader {
  id: string;
  fromName: string;
  fromAddress: string;
  subject: string;
  date: string;
  flags: MessageFlags;
}

export interface MessagePage {
  items: MessageHeader[];
  nextCursor: string | null;
  total: number;
  unread: number | null;
}

export interface Person {
  name: string;
  address: string;
}

export interface AttachmentInfo {
  name: string;
  mimeType: string;
  size: number;
}

export type MessageBody = { kind: "html"; html: string } | { kind: "text"; text: string } | { kind: "none" };

export interface MessageView {
  id: string;
  subject: string;
  fromName: string;
  fromAddress: string;
  to: Person[];
  cc: Person[];
  date: string;
  flags: MessageFlags;
  body: MessageBody;
  blockedRemoteImages: number;
  attachments: AttachmentInfo[];
}

export interface FlagResult {
  id: string;
  flags: MessageFlags;
}

export type AppErrorKind =
  | "AuthCancelled"
  | "AuthTimeout"
  | "AuthFailed"
  | "TokenExpired"
  | "ImapConnection"
  | "ImapAuth"
  | "Keyring"
  | "CursorExpired"
  | "MessageNotFound"
  | "MessageTooLarge"
  | "InvalidId"
  | "UnsupportedLink"
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
  listFolders: () => call<Folder[]>("list_folders"),
  listMessages: (folderId: string, cursor?: string | null) => call<MessagePage>("list_messages", { folderId, cursor: cursor ?? null }),
  getMessage: (messageId: string, allowRemoteImages: boolean) => call<MessageView>("get_message", { messageId, allowRemoteImages }),
  setFlags: (messageIds: string[], change: Partial<MessageFlags>) =>
    call<FlagResult[]>("set_flags", { messageIds, seen: change.seen ?? null, flagged: change.flagged ?? null }),
  openExternalLink: (url: string) => call<void>("open_external_link", { url }),
  disconnect: () => call<void>("disconnect"),
};

export function isAppError(error: unknown): error is AppError {
  return !!error && typeof error === "object" && "kind" in error;
}

/** Erros que significam "a conta precisa ser conectada de novo". */
export function isSessionLost(error: unknown): error is AppError {
  return isAppError(error) && (error.kind === "TokenExpired" || error.kind === "ImapAuth");
}

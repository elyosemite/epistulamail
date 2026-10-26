import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AccountStatus = "active" | "needs_reauth" | "signed_out";
export type AccountColor = "blue" | "teal" | "green" | "purple" | "magenta" | "orange";

export interface Account {
  id: string;
  provider: string;
  email: string;
  displayName: string;
  color: AccountColor;
  position: number;
  status: AccountStatus;
  /** Não lidas da Caixa de entrada. */
  unread: number | null;
  lastFolderId: string | null;
  /** Largura da lista em pixels, por conta (divisor entre lista e leitura). */
  listWidth: number | null;
  /** Foto do perfil do provedor (`data:` URL), baixada pelo backend (US5). */
  photo: string | null;
}

/** Intervalo em minutos; `null` = manualmente (US5, CA7). */
export type SyncMinutes = number | null;

export interface SyncSettings {
  defaultMinutes: SyncMinutes;
  accounts: Array<{ accountId: string; minutes: SyncMinutes; usesDefault: boolean }>;
}

/** Prazo das sinalizadas na coluna de pastas (US6, CA6). */
export type FlagBucket = "overdue" | "today" | "this_week" | "later" | "no_date";

export interface FlaggedCounts {
  overdue: number;
  today: number;
  thisWeek: number;
  later: number;
  noDate: number;
}

export type ThemeChoice = "light" | "dark" | "system";
/** Cor do fundo; "" é o padrão. */
export type CanvasChoice = "" | "blue" | "teal" | "green" | "purple" | "magenta" | "orange" | "sand";

/** Densidade da lista (US7, CA7): Confortável, Padrão ou Compacta. */
export type Density = "comfortable" | "standard" | "compact";

/** Aparência e preferências de lista, salvas no banco local (US6 e US7). */
export interface Appearance {
  theme: ThemeChoice;
  canvas: CanvasChoice;
  density: Density;
  showFolders: boolean;
  showReader: boolean;
}

/** Resultado de excluir, arquivar e mover em lote (US7, D1): o que saiu da pasta, o que ficou e o
 *  lote para o "Desfazer" (`null` quando nada ficou pendente, ex.: excluir de vez). */
export interface Moved {
  batch: string | null;
  ids: string[];
  failed: string[];
}

/** Seção 1 do menu da conta (US7, CA6). */
export interface AccountOverview {
  lastSyncAt: number | null;
  unread: number;
  saved: number;
  /** Tamanho do banco local, em bytes (todas as contas). */
  localBytes: number;
}

export interface SendRequest {
  accountId: string;
  kind: DraftKind;
  messageId: string | null;
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  body: string;
  /** Encaminhar: posições dos anexos da original que vão junto. */
  keepAttachments: number[];
}

export interface ProviderInfo {
  id: string;
  name: string;
  description: string;
  available: boolean;
  icon: string;
}

export interface Connected {
  account: Account;
  isNew: boolean;
  /** A conta já estava conectada e ativa. */
  wasActive: boolean;
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

/** Sinalização com prazo (US4): `due` em AAAA-MM-DD; `null` é "sem data". */
export interface Followup {
  due: string | null;
  completed: boolean;
}

export interface MessageHeader {
  id: string;
  fromName: string;
  fromAddress: string;
  subject: string;
  date: string;
  flags: MessageFlags;
  followup: Followup | null;
  /** Começo do texto (densidade Confortável); ausente enquanto não foi buscado. */
  snippet?: string;
  hasFiles: boolean;
  hasImages: boolean;
  /** Só na lista da pasta, com 2 ou mais mensagens: a conversa desta linha (US8, D1). */
  thread?: ThreadSummary;
  /** Só em resultados de busca: a pasta da mensagem. */
  folderName?: string;
  folderId?: string;
}

/** Uma linha da lista por conversa (US8, D1): quantas mensagens, quem participa e o estado do todo. */
export interface ThreadSummary {
  count: number;
  unread: boolean;
  flagged: boolean;
  /** Remetentes na ordem em que entraram na conversa, sem repetir. */
  participants: Array<{ name: string; mine: boolean }>;
}

/** Mensagem da conversa aberta (US8, CA6), da mais antiga para a mais nova. */
export interface ConversationMessage extends MessageHeader {
  /** Enviada pela própria conta ("Você"). */
  mine: boolean;
}

/** Notificações (US8, CA4). Horas cheias de 0 a 23. */
export interface NotifySettings {
  newMail: boolean;
  replies: boolean;
  due: boolean;
  dueHour: number;
  failures: boolean;
  reauth: boolean;
  offline: boolean;
  /** "full": remetente e assunto; "account": só a conta (D5). */
  privacy: "full" | "account";
  quiet: boolean;
  quietFrom: number;
  quietTo: number;
  trayOnClose: boolean;
  autostart: boolean;
  /** Contas sem notificação. */
  accountsOff: string[];
}

/** "Baixar e-mails para ler sem conexão" (US9, CA4). */
export type OfflineDays = "30" | "90" | "all" | "none";

export interface OfflineSettings {
  days: OfflineDays;
  /** Tamanho do banco local (todas as contas), em bytes. */
  localBytes: number;
  /** Conteúdo guardado fora do período, que pode ser apagado (US10, CA7). */
  outsideBytes: number;
}

/** Senha do Epistula e bloqueio (US10, CA2 e CA3). */
export interface LockStatus {
  locked: boolean;
  hasPassword: boolean;
  /** Segundos até poder tentar a senha de novo. */
  retryIn: number;
}

/** "Bloquear depois de" em minutos, ou "never" (D2: 15). */
export type LockMinutes = "5" | "15" | "30" | "60" | "never";

export interface LockSettings {
  minutes: LockMinutes;
  withWindows: boolean;
}

export interface PrivacySettings {
  hasPassword: boolean;
  lock: LockSettings;
  /** Espaço dos e-mails guardados por conta, em bytes. */
  stored: Array<{ accountId: string; bytes: number }>;
}

/** Para onde o clique numa notificação leva. */
export interface NoticeTarget {
  accountId: string;
  /** Sem ela, a Caixa de entrada da conta. */
  folderId: string | null;
  messageId: string | null;
  /** Abrir o login da conta ("Reconecte"). */
  reconnect?: boolean;
}

export type NoticeKind = "mail" | "reply" | "summary" | "due" | "failure" | "reauth" | "offline" | "info";

/** Uma notificação no histórico do sino (US8, CA7), com o texto que saiu no Windows. */
export interface Notice {
  id: number;
  accountId: string | null;
  kind: NoticeKind;
  title: string;
  line1: string;
  line2: string;
  /** Remetente, para o avatar (só com a privacidade "Remetente e assunto"). */
  sender: string | null;
  target: NoticeTarget | null;
  /** Segundos desde 1970. */
  createdAt: number;
  seen: boolean;
}

export interface NoticeList {
  items: Notice[];
  unseen: number;
  /** Não perturbe em efeito agora (horário ou 1 hora). */
  quiet: boolean;
  /** "Não perturbe por 1 hora" ligado. */
  snoozed: boolean;
}

export interface SearchPage {
  items: MessageHeader[];
  truncated: boolean;
}

export type DraftKind = "new" | "reply" | "reply_all" | "forward";

export interface DraftContent {
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  body: string;
}

export interface Draft extends DraftContent {
  id: number;
  kind: DraftKind;
  updatedAt: number;
}

export interface Quote {
  from: string;
  sent: string;
  to: string;
  cc: string;
  subject: string;
  text: string;
}

export interface ComposeTemplate {
  to: string[];
  cc: string[];
  subject: string;
  quote: Quote | null;
}

export type FollowupAction = { action: "set"; due: string | null } | { action: "complete" } | { action: "clear" };

export interface MessagePage {
  items: MessageHeader[];
  nextCursor: string | null;
  total: number;
  unread: number | null;
  /** Há uma sincronização desta pasta em andamento; o evento `mailbox-updated` avisa o fim. */
  syncing: boolean;
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
  /** O HTML tem texto citado, escondido até o "…" (US8, D3). */
  hasQuote: boolean;
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
  | "Config"
  | "Storage"
  | "AccountNotFound"
  | "ProviderUnavailable"
  | "SignedOut"
  | "InvalidInput"
  | "FolderExists"
  | "FolderNotFound"
  | "MessageMoving"
  | "SmtpConnection"
  | "SendRejected"
  | "NoRecipients"
  | "InvalidRecipient"
  | "AttachmentsTooLarge"
  | "Locked"
  | "WrongPassword"
  | "UnlockWait"
  | "WeakPassword";

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
  listAccounts: () => call<Account[]>("list_accounts"),
  /** A primeira tela apareceu: o backend libera a sincronização automática. */
  appReady: () => call<void>("app_ready"),
  listProviders: () => call<ProviderInfo[]>("list_providers"),
  connectAccount: (provider: string) => call<Connected>("connect_account", { provider }),
  reconnectAccount: (accountId: string) => call<Connected>("reconnect_account", { accountId }),
  cancelConnect: () => call<void>("cancel_connect"),
  updateAccount: (accountId: string, change: { displayName?: string; color?: AccountColor }) =>
    call<Account>("update_account", { accountId, displayName: change.displayName ?? null, color: change.color ?? null }),
  moveAccount: (accountId: string, direction: -1 | 1) => call<Account[]>("move_account", { accountId, direction }),
  /** `erase`: apaga também os dados da conta guardados neste computador (US10, CA7). */
  signOutAccount: (accountId: string, erase = false) => call<Account>("sign_out_account", { accountId, erase }),
  setLastFolder: (folderId: string) => call<void>("set_last_folder", { folderId }),
  listFolders: (accountId: string) => call<Folder[]>("list_folders", { accountId }),
  listMessages: (folderId: string, cursor?: string | null) => call<MessagePage>("list_messages", { folderId, cursor: cursor ?? null }),
  syncFolder: (folderId: string) => call<void>("sync_folder", { folderId }),
  getMessage: (messageId: string, allowRemoteImages: boolean) => call<MessageView>("get_message", { messageId, allowRemoteImages }),
  setFlags: (messageIds: string[], change: Partial<MessageFlags>) =>
    call<FlagResult[]>("set_flags", { messageIds, seen: change.seen ?? null, flagged: change.flagged ?? null }),
  openExternalLink: (url: string) => call<void>("open_external_link", { url }),
  searchMessages: (accountId: string, query: string) => call<SearchPage>("search_messages", { accountId, query }),
  moveMessages: (messageIds: string[], targetFolderId: string) => call<Moved>("move_messages", { messageIds, targetFolderId }),
  deleteMessages: (messageIds: string[]) => call<Moved>("delete_messages", { messageIds }),
  archiveMessages: (messageIds: string[]) => call<Moved>("archive_messages", { messageIds }),
  /** Desfaz o lote; devolve os ids que voltaram (os que já foram ao servidor não voltam). */
  undoMove: (accountId: string, batch: string) => call<string[]>("undo_move", { accountId, batch }),
  /** Agenda o envio (6 s para desfazer) e devolve o id; o resultado chega por `send-finished`. */
  sendMessage: (request: SendRequest) => call<string>("send_message", { request }),
  /** `false`: o envio já saiu. */
  cancelSend: (sendId: string) => call<boolean>("cancel_send", { sendId }),
  accountOverview: (accountId: string) => call<AccountOverview>("account_overview", { accountId }),
  syncAccount: (accountId: string) => call<void>("sync_account", { accountId }),
  createFolder: (accountId: string, name: string) => call<Folder>("create_folder", { accountId, name }),
  recentFolders: (accountId: string) => call<Folder[]>("recent_folders", { accountId }),
  setFollowup: (messageIds: string[], change: FollowupAction) =>
    call<Array<{ id: string; flags: MessageFlags; followup: Followup | null }>>("set_followup", {
      messageIds,
      action: change.action,
      due: change.action === "set" ? change.due : null,
    }),
  composeTemplate: (messageId: string, kind: Exclude<DraftKind, "new">) => call<ComposeTemplate>("compose_template", { messageId, kind }),
  getDraft: (accountId: string, kind: DraftKind, messageId: string | null) => call<Draft | null>("get_draft", { accountId, kind, messageId }),
  saveDraft: (accountId: string, kind: DraftKind, messageId: string | null, content: DraftContent) =>
    call<Draft>("save_draft", { accountId, kind, messageId, content }),
  deleteDraft: (accountId: string, kind: DraftKind, messageId: string | null) => call<void>("delete_draft", { accountId, kind, messageId }),
  setListWidth: (accountId: string, width: number) => call<void>("set_list_width", { accountId, width }),
  getSyncSettings: () => call<SyncSettings>("get_sync_settings"),
  flaggedCounts: (accountId: string, dates: { today: string; weekEnd: string }) =>
    call<FlaggedCounts>("flagged_counts", { accountId, today: dates.today, weekEnd: dates.weekEnd }),
  listFlagged: (accountId: string, bucket: FlagBucket, dates: { today: string; weekEnd: string }) =>
    call<SearchPage>("list_flagged", { accountId, bucket, today: dates.today, weekEnd: dates.weekEnd }),
  getAppearance: () => call<Appearance>("get_appearance"),
  setAppearance: (change: Partial<Appearance>) => call<Appearance>("set_appearance", { change }),
  setDefaultSyncInterval: (minutes: SyncMinutes) => call<SyncSettings>("set_default_sync_interval", { minutes }),
  setAccountSyncInterval: (accountIds: string[], minutes: SyncMinutes, useDefault: boolean) =>
    call<SyncSettings>("set_account_sync_interval", { accountIds, minutes, useDefault }),
  getConversation: (messageId: string) => call<ConversationMessage[]>("get_conversation", { messageId }),
  getNotifySettings: () => call<NotifySettings>("get_notify_settings"),
  setNotifySettings: (settings: NotifySettings) => call<NotifySettings>("set_notify_settings", { settings }),
  listNotices: () => call<NoticeList>("list_notices"),
  markNoticesSeen: () => call<void>("mark_notices_seen"),
  setSnooze: (on: boolean) => call<boolean>("set_snooze", { on }),
  /** Baixa em segundo plano as mensagens que o usuário deve abrir em seguida (US9, CA3). */
  prefetchMessages: (messageIds: string[]) => call<void>("prefetch_messages", { messageIds }),
  getOfflineSettings: () => call<OfflineSettings>("get_offline_settings"),
  setOfflineDays: (days: OfflineDays) => call<OfflineSettings>("set_offline_days", { days }),
  purgeOutsidePeriod: () => call<OfflineSettings>("purge_outside_period"),
  lockStatus: () => call<LockStatus>("lock_status"),
  unlockApp: (password: string) => call<void>("unlock_app", { password }),
  lockApp: () => call<void>("lock_app"),
  setAppPassword: (password: string) => call<void>("set_app_password", { password }),
  changeAppPassword: (current: string, password: string) => call<void>("change_app_password", { current, password }),
  removeAppPassword: (current: string) => call<void>("remove_app_password", { current }),
  /** "Esqueci a senha": apaga os dados deste computador e começa de novo, sem senha. */
  wipeLocalData: () => call<void>("wipe_local_data"),
  getPrivacySettings: () => call<PrivacySettings>("get_privacy_settings"),
  setLockSettings: (settings: LockSettings) => call<LockSettings>("set_lock_settings", { minutes: settings.minutes, withWindows: settings.withWindows }),
  /** O disco do Epistula está cifrado (BitLocker)? `null` quando não dá para saber. */
  diskEncryption: () => call<boolean | null>("disk_encryption"),
  /** Apaga deste computador os dados de uma conta de que você saiu. */
  deleteAccountData: (accountId: string) => call<void>("delete_account_data", { accountId }),
  rotateDbKey: () => call<void>("rotate_db_key"),
};

/** Eventos emitidos pelo backend (US3). */
export interface BackendEvents {
  "accounts-updated": null;
  "account-status": { accountId: string };
  "folders-updated": { accountId: string };
  "mailbox-updated": { accountId: string; folderId: string };
  "sync-status": { accountId: string; online: boolean };
  /** Mover que o servidor recusou: as mensagens voltaram para a origem (US7, D1). */
  "moves-finished": { accountId: string; failed: string[]; folderIds: string[] };
  "send-finished": { sendId: string; accountId: string; ok: boolean; error: AppError | null };
  /** Histórico do sino ou Não perturbe mudaram (US8, CA7). */
  "notices-updated": null;
  /** Clique numa notificação do Windows (US8, CA1). */
  "open-target": NoticeTarget;
  /** "Novo e-mail" no menu da bandeja (US8, CA5). */
  "tray-compose": null;
  /** Bloqueio, desbloqueio ou senha ligada e desligada (US10). */
  "lock-changed": LockStatus;
}

export function onBackendEvent<K extends keyof BackendEvents>(name: K, handler: (payload: BackendEvents[K]) => void): () => void {
  let unlisten: UnlistenFn | null = null;
  let disposed = false;
  listen<BackendEvents[K]>(name, (e) => handler(e.payload))
    .then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    })
    .catch(() => undefined);
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export function isAppError(error: unknown): error is AppError {
  return !!error && typeof error === "object" && "kind" in error;
}

/** Erros que significam "esta conta precisa entrar de novo" (acesso revogado, expirado ou saída). */
export function isAccessLost(error: unknown): error is AppError {
  return isAppError(error) && (error.kind === "TokenExpired" || error.kind === "ImapAuth" || error.kind === "SignedOut");
}

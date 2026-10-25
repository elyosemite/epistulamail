import type { Folder, FolderRole } from "./tauri";

const FOLDER_ICON: Record<FolderRole, string> = {
  inbox: "inbox",
  starred: "star",
  sent: "send",
  drafts: "file",
  all: "archive",
  spam: "spam",
  trash: "trash",
  custom: "folder",
};

export const folderIcon = (role: FolderRole) => FOLDER_ICON[role];

/** Destino válido para mover ou soltar: não é a pasta de origem nem "Com estrela" (a estrela do Gmail). */
export function canReceive(folder: Folder, sourceFolderId: string | null): boolean {
  return folder.id !== sourceFolderId && folder.role !== "starred";
}

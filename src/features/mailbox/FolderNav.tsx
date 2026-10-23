// Composição de DS: sidebar.html (.sidebar, .nav-group, .nav-heading, .nav-item) dentro de .app-shell__sidebar
import type { Folder, FolderRole } from "../../lib/tauri";
import { NavGroup, NavItem, NavSkeleton, Sidebar } from "../../ui/Sidebar";

const ICON: Record<FolderRole, string> = {
  inbox: "inbox",
  starred: "star",
  sent: "send",
  drafts: "file",
  all: "archive",
  spam: "spam",
  trash: "trash",
  custom: "folder",
};

interface FolderNavProps {
  folders: Folder[] | null;
  currentId: string | null;
  onSelect: (folder: Folder) => void;
}

export function FolderNav({ folders, currentId, onSelect }: FolderNavProps) {
  if (!folders) {
    return <Sidebar label="Pastas"><NavSkeleton /></Sidebar>;
  }
  const system = folders.filter((f) => f.role !== "custom");
  const labels = folders.filter((f) => f.role === "custom");
  const item = (f: Folder) => (
    <NavItem
      key={f.id}
      icon={ICON[f.role]}
      label={f.name}
      // Em Enviados, Rascunhos, Spam e Lixeira o contador de não lidas não ajuda; o DS mostra só onde há o que ler.
      count={f.role === "inbox" || f.role === "custom" ? f.unread : null}
      current={f.id === currentId}
      onSelect={() => onSelect(f)}
    />
  );
  return (
    <Sidebar label="Pastas">
      <NavGroup>{system.map(item)}</NavGroup>
      {labels.length > 0 && <NavGroup heading="Marcadores">{labels.map(item)}</NavGroup>}
    </Sidebar>
  );
}

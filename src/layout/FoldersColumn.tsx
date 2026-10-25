// Composição de DS: sidebar.html (.sidebar, .nav-group, .nav-heading, .nav-item; grupo "Sinalizadas") dentro de
// .app-shell__sidebar, no bloco de navegação
import { canReceive } from "../lib/folders";
import type { FlagBucket, Folder } from "../lib/tauri";
import { NavGroup, NavSkeleton, Sidebar } from "../ui/Sidebar";
import { FlaggedItem } from "./FlaggedItem";
import { FolderItem } from "./FolderItem";

interface FoldersColumnProps {
  folders: Folder[] | null;
  currentId: string | null;
  /** Prazos das sinalizadas com mensagens (vazio esconde o grupo). */
  flagged: ReadonlyArray<{ bucket: FlagBucket; label: string; count: number }>;
  currentFlag: FlagBucket | null;
  /** Arraste em andamento: pasta de origem (`null` na busca e nas sinalizadas) e pasta sob o ponteiro. */
  drag: { sourceId: string | null; hoveredId: string | null } | null;
  onSelect: (folder: Folder) => void;
  onSelectFlag: (bucket: FlagBucket) => void;
}

/** Coluna de pastas: as do sistema, os marcadores e, se houver, as sinalizadas por prazo. */
export function FoldersColumn({ folders, currentId, flagged, currentFlag, drag, onSelect, onSelectFlag }: FoldersColumnProps) {
  if (!folders) {
    return <Sidebar label="Pastas"><NavSkeleton /></Sidebar>;
  }
  const system = folders.filter((f) => f.role !== "custom");
  const labels = folders.filter((f) => f.role === "custom");
  const item = (f: Folder) => (
    <FolderItem
      key={f.id}
      folder={f}
      current={f.id === currentId}
      hovered={drag?.hoveredId === f.id}
      accepts={!!drag && canReceive(f, drag.sourceId)}
      onSelect={() => onSelect(f)}
    />
  );
  return (
    <Sidebar label="Pastas">
      <NavGroup>{system.map(item)}</NavGroup>
      {labels.length > 0 && <NavGroup heading="Marcadores">{labels.map(item)}</NavGroup>}
      {flagged.length > 0 && (
        <NavGroup heading="Sinalizadas">
          {flagged.map((f) => (
            <FlaggedItem key={f.bucket} bucket={f.bucket} label={f.label} count={f.count} current={f.bucket === currentFlag} onSelect={() => onSelectFlag(f.bucket)} />
          ))}
        </NavGroup>
      )}
    </Sidebar>
  );
}

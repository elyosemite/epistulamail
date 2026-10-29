// Composição de DS: pastas.html (.sidebar, .nav-group, .nav-heading, .nav-item; grupo "Sinalizadas") dentro de
// .app-shell__sidebar, no bloco de navegação; arquivos.html (US15): "Arquivo deste computador" no topo e o progresso
// da importação no fim (.meter com .progress, como o espaço da conta); busca.html "Buscas salvas" (US16):
// grupo próprio com .nav-item de ícone search e as não lidas
import { useState } from "react";
import { canReceive } from "../lib/folders";
import { progressShare } from "../lib/importing";
import { Icon } from "../ui/Icon";
import type { FlagBucket, Folder, SavedSearch } from "../lib/tauri";
import { Menu, type Anchor } from "../ui/Menu";
import { NavGroup, NavItem, NavSkeleton, Sidebar } from "../ui/Sidebar";
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
  /** Arquivo local (US15): a legenda no topo, no lugar do "Escrever". */
  local?: boolean;
  /** Importação deste arquivo local em andamento. */
  importing?: { imported: number; estimate: number } | null;
  /** Buscas salvas desta conta e as de todas as contas (US16). */
  saved?: SavedSearch[];
  currentSavedId?: number | null;
  onSelectSaved?: (saved: SavedSearch) => void;
  onDeleteSaved?: (saved: SavedSearch) => void;
}

/** Coluna de pastas: as do sistema, os marcadores e, se houver, as sinalizadas por prazo. */
export function FoldersColumn({ folders, currentId, flagged, currentFlag, drag, onSelect, onSelectFlag, local, importing, saved = [], currentSavedId, onSelectSaved, onDeleteSaved }: FoldersColumnProps) {
  const [menu, setMenu] = useState<{ saved: SavedSearch; anchor: Anchor } | null>(null);
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
      {local && (
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-100)", padding: "var(--space-100)", color: "var(--color-text-subtle)", font: "var(--font-body-small)" }}>
          <Icon name="hard-drive" size="sm" />
          Arquivo deste computador
        </div>
      )}
      <NavGroup>{system.map(item)}</NavGroup>
      {labels.length > 0 && <NavGroup heading="Marcadores">{labels.map(item)}</NavGroup>}
      {flagged.length > 0 && (
        <NavGroup heading="Sinalizadas">
          {flagged.map((f) => (
            <FlaggedItem key={f.bucket} bucket={f.bucket} label={f.label} count={f.count} current={f.bucket === currentFlag} onSelect={() => onSelectFlag(f.bucket)} />
          ))}
        </NavGroup>
      )}
      {saved.length > 0 && (
        <NavGroup heading="Buscas salvas">
          {saved.map((s) => (
            <NavItem
              key={s.id}
              icon="search"
              label={s.accountId === null ? `${s.name} (todas as contas)` : s.name}
              count={s.unread}
              current={s.id === currentSavedId}
              onSelect={() => onSelectSaved?.(s)}
              onMenu={onDeleteSaved && ((anchor) => setMenu({ saved: s, anchor }))}
            />
          ))}
        </NavGroup>
      )}
      {menu && onDeleteSaved && (
        <Menu
          label={`Busca salva ${menu.saved.name}`}
          anchor={menu.anchor}
          entries={[{ label: "Excluir busca salva", icon: "trash", onSelect: () => onDeleteSaved(menu.saved) }]}
          onClose={() => setMenu(null)}
        />
      )}
      {importing && (
        <div className="meter" role="progressbar" aria-label="Importação" aria-valuemin={0} aria-valuemax={importing.estimate || undefined} aria-valuenow={importing.imported}>
          <div className="progress"><span style={{ width: `${progressShare(importing.imported, importing.estimate)}%` }} /></div>
          {importing.estimate > 0
            ? `Importando ${importing.imported.toLocaleString("pt-BR")} de ${importing.estimate.toLocaleString("pt-BR")}`
            : `Importando ${importing.imported.toLocaleString("pt-BR")}`}
        </div>
      )}
    </Sidebar>
  );
}

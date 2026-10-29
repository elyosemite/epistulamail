// DS: pastas.html · .nav-item (aria-current, .has-unread, __count) e, ao arrastar mensagens, .is-drop-target /
//     .is-drop-blocked (US5, CA3; arraste por ponteiro na US6)
import { folderIcon } from "../lib/folders";
import type { Folder } from "../lib/tauri";
import { NavItem } from "../ui/Sidebar";

interface FolderItemProps {
  folder: Folder;
  current: boolean;
  /** Mensagens estão sendo arrastadas e o ponteiro está sobre esta pasta. */
  hovered: boolean;
  /** Pode receber as mensagens arrastadas (não é a origem nem "Com estrela"). */
  accepts: boolean;
  onSelect: () => void;
}

/** Uma pasta: Caixa de entrada, Com estrela, Enviados, Rascunhos, Lixeira ou um marcador. */
export function FolderItem({ folder, current, hovered, accepts, onSelect }: FolderItemProps) {
  return (
    <NavItem
      icon={folderIcon(folder.role)}
      label={folder.name}
      // Em Enviados, Rascunhos, Spam e Lixeira o contador de não lidas não ajuda; o DS mostra só onde há o que ler.
      count={folder.role === "inbox" || folder.role === "custom" ? folder.unread : null}
      current={current}
      onSelect={onSelect}
      dropId={folder.id}
      drop={hovered ? (accepts ? "target" : "blocked") : null}
    />
  );
}

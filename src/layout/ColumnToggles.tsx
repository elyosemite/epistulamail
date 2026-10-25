// DS: espacamento.html "Colunas recolhíveis" · botões .btn--subtle.btn--icon com os ícones sidebar-left e sidebar-right,
//     aria-pressed com a coluna visível (L43)
import { IconButton } from "../ui/Button";

interface ColumnTogglesProps {
  showFolders: boolean;
  showReader: boolean;
  onToggleFolders: () => void;
  onToggleReader: () => void;
}

/** Mostrar e esconder a coluna de pastas (Ctrl+B) e a de leitura (Ctrl+Alt+B), no fim do header. */
export function ColumnToggles({ showFolders, showReader, onToggleFolders, onToggleReader }: ColumnTogglesProps) {
  return (
    <>
      <IconButton
        icon="sidebar-left"
        label="Pastas"
        aria-pressed={showFolders}
        data-tip={`${showFolders ? "Esconder" : "Mostrar"} pastas (Ctrl+B)`}
        onClick={onToggleFolders}
      />
      <IconButton
        icon="sidebar-right"
        label="Leitura"
        aria-pressed={showReader}
        data-tip={`${showReader ? "Esconder" : "Mostrar"} leitura (Ctrl+Alt+B)`}
        onClick={onToggleReader}
      />
    </>
  );
}

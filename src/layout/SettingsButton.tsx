// DS: header.html "Configurações" · .btn--subtle.btn--icon com o ícone settings, aria-pressed com as configurações abertas
import { Icon } from "../ui/Icon";

export function SettingsButton({ open, onClick }: { open: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      className="btn btn--subtle btn--icon"
      aria-label="Configurações"
      aria-pressed={open}
      data-tip="Configurações (Ctrl+,)"
      onClick={onClick}
    >
      <Icon name="settings" />
    </button>
  );
}

// Composição de DS: modais.html "Configurações → Geral" (.settings__group, .setting com .segmented e .toggle)
import { DENSITY_CHOICES } from "../../lib/appearance";
import type { Appearance } from "../../lib/tauri";
import { Segmented } from "../../ui/Segmented";
import { Setting, SettingsGroup } from "../../ui/SettingsSheet";

/** Geral (US7, CA4 e D7): densidade da lista e colunas visíveis, os mesmos do menu da conta e do header. */
export function GeneralSettings({ appearance, onChange }: { appearance: Appearance; onChange: (change: Partial<Appearance>) => void }) {
  return (
    <SettingsGroup title="Lista de e-mails">
      <Setting labelId="general-density" label="Densidade da lista" description="Quanto de cada e-mail aparece na lista. Vale para todas as contas.">
        <Segmented label="Densidade da lista" labelledBy="general-density" options={DENSITY_CHOICES} value={appearance.density} onChange={(density) => onChange({ density })} />
      </Setting>
      <Setting labelId="general-folders" label="Mostrar a coluna de pastas" description="Também pelo botão no header ou Ctrl+B.">
        <input type="checkbox" className="toggle" aria-labelledby="general-folders" checked={appearance.showFolders} onChange={(e) => onChange({ showFolders: e.target.checked })} />
      </Setting>
      <Setting labelId="general-reader" label="Mostrar a coluna de leitura" description="Também pelo botão no header ou Ctrl+Alt+B.">
        <input type="checkbox" className="toggle" aria-labelledby="general-reader" checked={appearance.showReader} onChange={(e) => onChange({ showReader: e.target.checked })} />
      </Setting>
    </SettingsGroup>
  );
}

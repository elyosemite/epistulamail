// Composição de DS: modais.html "Configurações" (.settings__group, .setting com .select), campos.html (.select)
import { useEffect, useState } from "react";
import { formatBytes } from "../../lib/format";
import { api, type OfflineDays, type OfflineSettings as Settings } from "../../lib/tauri";
import { Select } from "../../ui/Select";
import { Setting, SettingsGroup } from "../../ui/SettingsSheet";
import { useToast } from "../../ui/Toast";

const CHOICES: Array<{ value: OfflineDays; label: string }> = [
  { value: "30", label: "Últimos 30 dias" },
  { value: "90", label: "Últimos 90 dias" },
  { value: "all", label: "Tudo" },
  { value: "none", label: "Só ao abrir" },
];

/** E-mail → Ler sem conexão (US9, CA4): até onde baixar os e-mails antes de abrir e o espaço ocupado. */
export function OfflineSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const toast = useToast();

  useEffect(() => {
    api.getOfflineSettings().then(setSettings).catch(() => undefined);
  }, []);

  const change = (days: OfflineDays) => {
    const before = settings;
    if (before) setSettings({ ...before, days });
    api.setOfflineDays(days)
      .then(setSettings)
      .catch(() => {
        setSettings(before);
        toast({ kind: "error", title: "Não foi possível salvar", description: "Tente de novo em instantes." });
      });
  };

  return (
    <SettingsGroup title="Ler sem conexão">
      <Setting
        labelId="offline-days"
        label="Baixar e-mails para ler sem conexão"
        description={
          settings
            ? `Os e-mails desse período ficam neste computador e abrem na hora. Os maiores que 2 MB baixam ao abrir. Ocupado agora: ${formatBytes(settings.localBytes)}.`
            : "Os e-mails desse período ficam neste computador e abrem na hora."
        }
      >
        <Select labelledBy="offline-days" value={settings?.days ?? "30"} options={CHOICES} disabled={!settings} onChange={(v) => change(v as OfflineDays)} />
      </Setting>
    </SettingsGroup>
  );
}

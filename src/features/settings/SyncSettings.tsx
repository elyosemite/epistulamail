// Composição de DS: modais.html "Configurações" (.settings__group, .setting, .setting-list com ação em lote),
// campos.html (.select), avatar.html (.avatar com foto), mensagens.html (.section-message de erro)
import { useEffect, useId, useState } from "react";
import { accountInitial, avatarColorClass } from "../../lib/accounts";
import { emptySelection, masterState, toggle, toggleAll, type Selection } from "../../lib/selection";
import { decodeChoice, encodeChoice, intervalLabel, intervalShort, SYNC_CHOICES } from "../../lib/syncInterval";
import { api, type Account, type SyncMinutes, type SyncSettings as Settings } from "../../lib/tauri";
import { AccountAvatar } from "../../ui/AccountAvatar";
import { Button } from "../../ui/Button";
import { SectionMessage } from "../../ui/SectionMessage";
import { Select, type SelectOption } from "../../ui/Select";
import { Setting, SettingList, SettingListHead, SettingListRow, SettingNote, SettingsGroup } from "../../ui/SettingsSheet";
import { Spinner } from "../../ui/Spinner";
import { useToast } from "../../ui/Toast";

const OPTIONS: SelectOption[] = SYNC_CHOICES.map((m) => ({ value: encodeChoice(m), label: intervalLabel(m) }));

/** E-mail → Sincronização (US5, CA7): padrão para todas, intervalo por conta e aplicar a várias. */
export function SyncSettings({ accounts }: { accounts: Account[] }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState(false);
  const [checked, setChecked] = useState<Selection>(emptySelection);
  const [bulk, setBulk] = useState("default");
  const toast = useToast();
  const defaultId = useId();

  useEffect(() => {
    api.getSyncSettings().then(setSettings).catch(() => setError(true));
  }, []);

  const save = async (run: () => Promise<Settings>) => {
    try {
      setSettings(await run());
    } catch {
      toast({ kind: "error", title: "Não foi possível salvar o intervalo", description: "Tente de novo em instantes." });
    }
  };

  if (error) {
    return <SectionMessage appearance="error" title="Não foi possível carregar a sincronização">Feche e abra as configurações de novo.</SectionMessage>;
  }
  if (!settings) return <Spinner label="Carregando" />;

  const usable = accounts.filter((a) => a.status !== "signed_out").map((a) => a.id);
  const master = masterState(checked, usable);
  const count = usable.filter((id) => checked.ids.has(id)).length;
  const withDefault: SelectOption[] = [{ value: "default", label: `Usar o padrão (${intervalShort(settings.defaultMinutes)})` }, ...OPTIONS];

  const setAccounts = (ids: string[], value: string) => {
    const choice = decodeChoice(value);
    const minutes: SyncMinutes = choice.useDefault ? null : choice.minutes;
    void save(() => api.setAccountSyncInterval(ids, minutes, choice.useDefault));
  };

  return (
    <SettingsGroup title="Sincronização">
      <Setting labelId={defaultId} label="Verificar novos e-mails a cada" description="Vale para todas as contas que usam o padrão.">
        <Select
          labelledBy={defaultId}
          value={encodeChoice(settings.defaultMinutes)}
          options={OPTIONS}
          onChange={(v) => {
            const choice = decodeChoice(v);
            if (!choice.useDefault) void save(() => api.setDefaultSyncInterval(choice.minutes));
          }}
        />
      </Setting>
      <SettingList
        head={
          <SettingListHead
            selecting={count > 0}
            master={master}
            masterLabel={master === "all" ? "Desmarcar todas as contas" : "Selecionar todas as contas"}
            disabled={usable.length === 0}
            onMaster={() => setChecked((sel) => toggleAll(sel, usable))}
            bulk={
              count > 0 && (
                <>
                  <Select label="Intervalo para as contas selecionadas" value={bulk} options={withDefault} onChange={setBulk} />
                  <Button
                    appearance="primary"
                    compact
                    onClick={() => {
                      setAccounts(usable.filter((id) => checked.ids.has(id)), bulk);
                      setChecked(emptySelection);
                    }}
                  >
                    Aplicar
                  </Button>
                </>
              )
            }
          >
            {count > 0 ? `Aplicar a ${count} ${count === 1 ? "conta" : "contas"}` : "Por conta"}
          </SettingListHead>
        }
      >
        {accounts.map((a) => {
          const own = settings.accounts.find((s) => s.accountId === a.id);
          const signedOut = a.status === "signed_out";
          return (
            <SettingListRow
              key={a.id}
              checked={checked.ids.has(a.id)}
              disabled={signedOut}
              checkLabel={`Selecionar ${a.displayName}`}
              onCheck={() => setChecked((sel) => toggle(sel, usable, a.id, false))}
              media={<AccountAvatar initial={accountInitial(a.displayName)} colorClass={avatarColorClass(a.color)} photo={a.photo} />}
              title={a.displayName}
              subtitle={signedOut ? "Você saiu desta conta" : a.email}
              control={
                <Select
                  label={`Intervalo de ${a.displayName}`}
                  value={!own || own.usesDefault ? "default" : encodeChoice(own.minutes)}
                  options={withDefault}
                  disabled={signedOut}
                  onChange={(v) => setAccounts([a.id], v)}
                />
              }
            />
          );
        })}
      </SettingList>
      <SettingNote>"Manualmente": a conta só procura e-mails novos quando você a abre ou clica em Atualizar.</SettingNote>
    </SettingsGroup>
  );
}

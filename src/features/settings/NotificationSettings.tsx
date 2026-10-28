// Composição de DS: modais.html "Configurações → Notificações" (US8, CA4: .settings__group, .setting com .toggle, .select e
// .segmented; .setting-list com .setting-list__row--toggle por conta)
import { useCallback, useEffect, useRef, useState } from "react";
import { accountInitial, avatarColorClass } from "../../lib/accounts";
import { api, type Account, type NotifySettings } from "../../lib/tauri";
import { AccountAvatar } from "../../ui/AccountAvatar";
import { Segmented } from "../../ui/Segmented";
import { Select } from "../../ui/Select";
import { Setting, SettingList, SettingsGroup, SettingNote, SettingToggleRow } from "../../ui/SettingsSheet";
import { Spinner } from "../../ui/Spinner";
import { useToast } from "../../ui/Toast";

const HOURS = Array.from({ length: 24 }, (_, h) => ({ value: String(h), label: `${h}:00` }));
const PRIVACY = [
  { value: "account" as const, label: "Só a conta" },
  { value: "full" as const, label: "Remetente e assunto" },
];

/** Notificações (US8, CA4): o que avisa, por conta, privacidade, Não perturbe e a bandeja. */
export function NotificationSettings({ accounts }: { accounts: Account[] }) {
  const [settings, setSettings] = useState<NotifySettings | null>(null);
  const [platform, setPlatform] = useState<"windows" | "linux" | "macos">("windows");
  const toast = useToast();

  useEffect(() => {
    api.getNotifySettings().then(setSettings).catch(() => toast({ kind: "error", title: "Não foi possível carregar as notificações" }));
    api.platform().then(setPlatform).catch(() => undefined);
  }, [toast]);

  const current = useRef(settings);
  current.current = settings;
  const change = useCallback((patch: Partial<NotifySettings>) => {
    const before = current.current;
    if (!before) return;
    const next = { ...before, ...patch };
    setSettings(next);
    api.setNotifySettings(next)
      .then(setSettings)
      .catch(() => {
        setSettings(before);
        toast({ kind: "error", title: "Não foi possível salvar as notificações" });
      });
  }, [toast]);

  if (!settings) return <Spinner label="Carregando as notificações" />;

  const toggle = (key: keyof NotifySettings, labelId: string) => (
    <input type="checkbox" className="toggle" aria-labelledby={labelId} checked={settings[key] as boolean} onChange={(e) => change({ [key]: e.target.checked })} />
  );
  const accountOn = (id: string) => !settings.accountsOff.includes(id);

  return (
    <>
      <SettingsGroup title="Avisar quando">
        <Setting labelId="notify-new" label="Chegar um e-mail novo" description="Na Caixa de entrada. Mais de 3 de uma vez viram um resumo.">
          {toggle("newMail", "notify-new")}
        </Setting>
        <Setting labelId="notify-replies" label="Alguém responder um e-mail seu" description="Avisa mesmo com os e-mails novos desligados.">
          {toggle("replies", "notify-replies")}
        </Setting>
        <Setting labelId="notify-due" label="Uma sinalizada vencer" description="Um aviso por dia com as que vencem hoje e as atrasadas.">
          <div className="toolbar__end">
            <Select label="Horário do aviso dos prazos" value={String(settings.dueHour)} options={HOURS} disabled={!settings.due} onChange={(h) => change({ dueHour: Number(h) })} />
            {toggle("due", "notify-due")}
          </div>
        </Setting>
        <Setting labelId="notify-failures" label="Um envio ou um mover falhar" description="Quando a janela não está em foco.">
          {toggle("failures", "notify-failures")}
        </Setting>
        <Setting labelId="notify-reauth" label="Uma conta precisar reconectar">
          {toggle("reauth", "notify-reauth")}
        </Setting>
        <Setting labelId="notify-offline" label="Ficar sem conexão por mais de 1 hora">
          {toggle("offline", "notify-offline")}
        </Setting>
        {platform === "linux" && (
          <SettingNote>No Linux, as notificações do sistema chegam numa próxima versão. O sino e o histórico continuam funcionando.</SettingNote>
        )}
      </SettingsGroup>
      <SettingsGroup title="Contas">
        <SettingList>
          {accounts.map((a) => (
            <SettingToggleRow
              key={a.id}
              media={<AccountAvatar initial={accountInitial(a.displayName)} colorClass={avatarColorClass(a.color)} photo={a.photo} />}
              title={a.displayName}
              subtitle={a.email}
              control={
                <input
                  type="checkbox"
                  className="toggle"
                  aria-label={`Notificações de ${a.displayName}`}
                  checked={accountOn(a.id)}
                  onChange={(e) => change({ accountsOff: e.target.checked ? settings.accountsOff.filter((id) => id !== a.id) : [...settings.accountsOff, a.id] })}
                />
              }
            />
          ))}
        </SettingList>
      </SettingsGroup>
      <SettingsGroup title="Privacidade">
        <Setting labelId="notify-privacy" label="O que aparece na notificação" description={'As notificações aparecem na tela bloqueada, e o sistema guarda o texto delas sem cifra. "Só a conta" não deixa assunto nem remetente fora do EpistulaMail.'}>
          <Segmented label="O que aparece na notificação" labelledBy="notify-privacy" options={PRIVACY} value={settings.privacy} onChange={(privacy) => change({ privacy })} />
        </Setting>
      </SettingsGroup>
      <SettingsGroup title="Não perturbe">
        <Setting labelId="notify-quiet" label="Silenciar todo dia" description="Ao terminar, um resumo do que chegou.">
          <div className="toolbar__end">
            <Select label="Das" value={String(settings.quietFrom)} options={HOURS} disabled={!settings.quiet} onChange={(h) => change({ quietFrom: Number(h) })} />
            <span className="setting__desc">às</span>
            <Select label="Até" value={String(settings.quietTo)} options={HOURS} disabled={!settings.quiet} onChange={(h) => change({ quietTo: Number(h) })} />
            {toggle("quiet", "notify-quiet")}
          </div>
        </Setting>
      </SettingsGroup>
      <SettingsGroup title="Ao fechar a janela">
        <Setting labelId="notify-tray" label="Continuar na bandeja do sistema" description="Para receber notificações com a janela fechada. Para sair, use Sair no ícone da bandeja.">
          {toggle("trayOnClose", "notify-tray")}
        </Setting>
        <Setting labelId="notify-autostart" label="Abrir o EpistulaMail ao iniciar o sistema" description="Abre direto na bandeja, sem mostrar a janela.">
          {toggle("autostart", "notify-autostart")}
        </Setting>
      </SettingsGroup>
    </>
  );
}

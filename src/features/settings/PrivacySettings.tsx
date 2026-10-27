// Composição de DS: modais.html "Configurações → Privacidade" (US10: .settings__group, .setting com .toggle, .select e
// .btn--subtle; .section-message--warning do BitLocker; .setting-list__row--toggle com .setting-list__end e
// .setting-list__size, L56) e os modais de PrivacyDialogs
import { useCallback, useEffect, useState } from "react";
import { useAppLock } from "../lock/LockGate";
import { accountInitial, avatarColorClass } from "../../lib/accounts";
import { formatBytes } from "../../lib/format";
import { LOCK_CHOICES } from "../../lib/lock";
import { api, type Account, type LockMinutes, type PrivacySettings as Settings } from "../../lib/tauri";
import { AccountAvatar } from "../../ui/AccountAvatar";
import { Button } from "../../ui/Button";
import { SectionMessage } from "../../ui/SectionMessage";
import { Select } from "../../ui/Select";
import { Setting, SettingList, SettingNote, SettingsGroup, SettingToggleRow } from "../../ui/SettingsSheet";
import { Spinner } from "../../ui/Spinner";
import { useToast } from "../../ui/Toast";
import { ChangePasswordDialog, DeleteAccountDataDialog, RemovePasswordDialog, RotateKeyDialog, SetPasswordDialog } from "./PrivacyDialogs";

type Dialog = { kind: "set" } | { kind: "change" } | { kind: "remove" } | { kind: "rotate" } | { kind: "delete"; account: Account; bytes: number } | null;

/** Privacidade (US10): senha do Epistula, bloqueio, chave de proteção, BitLocker e o que está guardado por conta. */
export function PrivacySettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  // Todas as contas, inclusive as de que você saiu: é aqui que se apagam os dados delas.
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [diskEncrypted, setDiskEncrypted] = useState<boolean | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const { setIdleMinutes } = useAppLock();
  const toast = useToast();

  const load = useCallback(() => {
    api.getPrivacySettings().then(setSettings).catch(() => toast({ kind: "error", title: "Não foi possível carregar a privacidade" }));
    api.listAccounts().then(setAccounts).catch(() => undefined);
  }, [toast]);

  useEffect(() => {
    load();
    api.diskEncryption().then(setDiskEncrypted).catch(() => undefined);
  }, [load]);

  if (!settings) return <Spinner label="Carregando a privacidade" />;

  const saveLock = (change: Partial<Settings["lock"]>) => {
    const before = settings;
    const lock = { ...settings.lock, ...change };
    setSettings({ ...settings, lock });
    api.setLockSettings(lock)
      .then((saved) => {
        setSettings((s) => (s ? { ...s, lock: saved } : s));
        setIdleMinutes(saved.minutes);
      })
      .catch(() => {
        setSettings(before);
        toast({ kind: "error", title: "Não foi possível salvar", description: "Tente de novo em instantes." });
      });
  };

  const bytesOf = (id: string) => settings.stored.find((s) => s.accountId === id)?.bytes ?? 0;

  return (
    <>
      <SettingsGroup title="Senha do EpistulaMail">
        <Setting
          labelId="privacy-password"
          label="Pedir uma senha ao abrir o EpistulaMail"
          description="Protege as mensagens guardadas neste computador até de quem usa o seu Windows. Desligada, quem entra no seu Windows abre o EpistulaMail."
        >
          <input
            type="checkbox"
            className="toggle"
            aria-labelledby="privacy-password"
            checked={settings.hasPassword}
            onChange={() => setDialog({ kind: settings.hasPassword ? "remove" : "set" })}
          />
        </Setting>
        {settings.hasPassword && (
          <>
            <Setting labelId="privacy-idle" label="Bloquear depois de" description="Sem mexer no EpistulaMail por esse tempo.">
              <Select labelledBy="privacy-idle" value={settings.lock.minutes} options={LOCK_CHOICES} onChange={(v) => saveLock({ minutes: v as LockMinutes })} />
            </Setting>
            <Setting labelId="privacy-windows" label="Bloquear junto com o Windows" description="Quando a tela do Windows bloquear (Win+L).">
              <input type="checkbox" className="toggle" aria-labelledby="privacy-windows" checked={settings.lock.withWindows} onChange={(e) => saveLock({ withWindows: e.target.checked })} />
            </Setting>
            <Setting labelId="privacy-change" label="Senha" description="Trocar a senha não baixa nada de novo.">
              <Button appearance="subtle" icon="key" aria-describedby="privacy-change" onClick={() => setDialog({ kind: "change" })}>Trocar senha</Button>
            </Setting>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup title="Proteção dos dados">
        <Setting
          labelId="privacy-rotate"
          label="Trocar a chave de proteção dos dados"
          description="Cifra de novo tudo que está guardado neste computador com uma chave nova. Uma cópia antiga do arquivo deixa de abrir. Leva alguns segundos."
        >
          <Button appearance="subtle" icon="refresh" aria-describedby="privacy-rotate" onClick={() => setDialog({ kind: "rotate" })}>Trocar chave</Button>
        </Setting>
        {diskEncrypted === false && (
          <SectionMessage appearance="warning" title="Este disco não está cifrado">
            O Windows pode copiar partes da memória do EpistulaMail para o disco ao hibernar ou com pouca memória. Ligue a criptografia do disco
            (BitLocker) nas Configurações do Windows, em Privacidade e segurança.
          </SectionMessage>
        )}
      </SettingsGroup>

      <SettingsGroup title="Guardado neste computador">
        <SettingList>
          {accounts.map((a) => {
            const signedOut = a.status === "signed_out";
            const local = a.status === "local";
            const bytes = bytesOf(a.id);
            return (
              <SettingToggleRow
                key={a.id}
                disabled={signedOut}
                media={<AccountAvatar initial={accountInitial(a.displayName)} colorClass={avatarColorClass(a.color)} photo={a.photo} archive={local} />}
                title={a.displayName}
                subtitle={signedOut ? "Você saiu desta conta" : local ? "Arquivo deste computador" : a.email}
                control={
                  <span className="setting-list__end">
                    <span className="setting-list__size">{formatBytes(bytes)}</span>
                    {signedOut && (
                      <Button appearance="subtle" compact aria-label={`Apagar os dados de ${a.displayName}`} onClick={() => setDialog({ kind: "delete", account: a, bytes })}>
                        Apagar
                      </Button>
                    )}
                  </span>
                }
              />
            );
          })}
        </SettingList>
        <SettingNote>
          Para apagar os dados de uma conta que você usa, saia dela e marque "Apagar também os e-mails desta conta". Apagar troca a chave de
          proteção. Nada muda no servidor.
        </SettingNote>
      </SettingsGroup>

      {dialog?.kind === "set" && (
        <SetPasswordDialog
          onConfirm={async (password) => {
            await api.setAppPassword(password);
            load();
            toast({ kind: "success", title: "Senha do EpistulaMail ligada" });
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "change" && (
        <ChangePasswordDialog
          onConfirm={async (current, password) => {
            await api.changeAppPassword(current, password);
            toast({ kind: "success", title: "Senha trocada" });
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "remove" && (
        <RemovePasswordDialog
          onConfirm={async (current) => {
            await api.removeAppPassword(current);
            load();
            toast({ kind: "info", title: "Senha do EpistulaMail desligada" });
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "rotate" && (
        <RotateKeyDialog
          onConfirm={async () => {
            try {
              await api.rotateDbKey();
              toast({ kind: "success", title: "Chave de proteção trocada" });
            } catch {
              toast({ kind: "error", title: "Não foi possível trocar a chave", description: "Os dados continuam protegidos pela chave atual." });
            }
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "delete" && (
        <DeleteAccountDataDialog
          account={dialog.account}
          bytes={dialog.bytes}
          onConfirm={async () => {
            try {
              await api.deleteAccountData(dialog.account.id);
              toast({ kind: "success", title: `Dados de ${dialog.account.displayName} apagados deste computador` });
            } catch {
              toast({ kind: "error", title: "Não foi possível apagar os dados", description: "Tente de novo em instantes." });
            }
            load();
          }}
          onClose={() => setDialog(null)}
        />
      )}
    </>
  );
}

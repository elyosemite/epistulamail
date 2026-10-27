// Composição de DS: arquivos.html "Configurações → Geral" (.settings__group com .setting-list__row--toggle,
// .setting-list__end e __size, o avatar L71 e o botão de abrir outro arquivo) · modais.html (remover)
import { useCallback, useEffect, useState } from "react";
import { formatBytes } from "../../lib/format";
import { FORMAT_LABEL } from "../../lib/importing";
import { api, onBackendEvent, type Account, type Archive } from "../../lib/tauri";
import { AccountAvatar } from "../../ui/AccountAvatar";
import { Button } from "../../ui/Button";
import { SettingList, SettingsGroup, SettingToggleRow } from "../../ui/SettingsSheet";
import { useToast } from "../../ui/Toast";
import { RemoveArchiveDialog } from "../accounts/AccountDialogs";

function importedOn(at: number): string {
  return new Date(at * 1000).toLocaleDateString("pt-BR", { day: "numeric", month: "short", year: "numeric" }).replace(".", "");
}

/** Arquivos de e-mail (US15, CA1): os arquivos locais, o espaço de cada um, remover e abrir outro. */
export function ArchiveSettings({ accounts, onOpenFile }: { accounts: Account[]; onOpenFile: () => void }) {
  const [archives, setArchives] = useState<Archive[]>([]);
  const [removing, setRemoving] = useState<Account | null>(null);
  const toast = useToast();

  const load = useCallback(() => {
    api.listArchives().then(setArchives).catch(() => setArchives([]));
  }, []);
  useEffect(() => {
    load();
    const offs = [onBackendEvent("accounts-updated", load), onBackendEvent("import-finished", load)];
    return () => offs.forEach((off) => off());
  }, [load]);

  return (
    <SettingsGroup title="Arquivos de e-mail">
      {archives.length > 0 && (
        <SettingList>
          {archives.map((a) => {
            const account = accounts.find((x) => x.id === a.accountId);
            const name = account?.displayName ?? a.sourceName;
            const state = a.running ? "importando" : a.state === "interrupted" ? "importação interrompida" : `importado em ${importedOn(a.finishedAt ?? a.createdAt)}`;
            return (
              <SettingToggleRow
                key={a.accountId}
                media={<AccountAvatar initial="" colorClass={null} photo={null} archive />}
                title={name}
                subtitle={`${FORMAT_LABEL[a.format]} · ${state}`}
                control={
                  <span className="setting-list__end">
                    <span className="setting-list__size">{formatBytes(a.bytes)}</span>
                    <Button appearance="subtle" compact disabled={a.running || !account} aria-label={`Remover ${name}`} onClick={() => account && setRemoving(account)}>
                      Remover
                    </Button>
                  </span>
                }
              />
            );
          })}
        </SettingList>
      )}
      <div style={{ marginTop: archives.length > 0 ? "var(--space-150)" : 0 }}>
        <Button appearance="subtle" icon="plus" onClick={onOpenFile}>Abrir arquivo de e-mail…</Button>
      </div>
      {removing && (
        <RemoveArchiveDialog
          account={removing}
          onConfirm={async () => {
            try {
              await api.removeArchive(removing.id);
              toast({ kind: "info", title: `${removing.displayName} saiu do EpistulaMail`, description: "As mensagens foram apagadas deste computador." });
            } catch {
              toast({ kind: "error", title: "Não foi possível remover", description: "Tente de novo em instantes." });
            }
            load();
          }}
          onClose={() => setRemoving(null)}
        />
      )}
    </SettingsGroup>
  );
}

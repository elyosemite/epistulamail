// Composição de DS: criptografia.html "Configurações → Criptografia" · .settings__group, .choice-list (L64) com
// .fingerprint--compact (L65), .setting com .toggle, .segmented e .select, .section-message
import { useCallback, useEffect, useState } from "react";
import { keyDate, keyName } from "../../lib/crypto";
import { api, type Account, type CryptoSettings as Settings, type PgpKey } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { ChoiceList } from "../../ui/ChoiceCard";
import { Fingerprint } from "../../ui/Fingerprint";
import { Lozenge } from "../../ui/Lozenge";
import { SectionMessage } from "../../ui/SectionMessage";
import { Segmented } from "../../ui/Segmented";
import { Select } from "../../ui/Select";
import { Setting, SettingNote, SettingsGroup } from "../../ui/SettingsSheet";
import { Spinner } from "../../ui/Spinner";
import { useToast } from "../../ui/Toast";
import { AddKeyDialog } from "./AddKeyDialog";

const NONE = "none";

const SEARCH_CHOICES = [
  { value: "ask", label: "Perguntar antes" },
  { value: "always", label: "Sempre" },
  { value: "never", label: "Nunca" },
];

interface CryptoSettingsProps {
  accounts: Account[];
  initialAccountId: string;
  onManageKeys: (accountId: string) => void;
}

/** Criptografia (US13, CA1): a chave de cada conta e como as mensagens saem. */
export function CryptoSettings({ accounts, initialAccountId, onManageKeys }: CryptoSettingsProps) {
  const [accountId, setAccountId] = useState(accounts.some((a) => a.id === initialAccountId) ? initialAccountId : accounts[0]?.id ?? "");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [keys, setKeys] = useState<PgpKey[]>([]);
  const [adding, setAdding] = useState(false);
  const toast = useToast();
  const account = accounts.find((a) => a.id === accountId);

  const load = useCallback(() => {
    if (!accountId) return;
    api.getCryptoSettings(accountId).then(setSettings).catch(() => toast({ kind: "error", title: "Não foi possível carregar a criptografia" }));
    api.listPgpKeys().then(setKeys).catch(() => setKeys([]));
  }, [accountId, toast]);
  useEffect(load, [load]);

  if (!account) return <SettingNote>Conecte uma conta para usar criptografia.</SettingNote>;
  if (!settings) return <Spinner label="Carregando a criptografia" />;

  const save = (change: Partial<Settings>) => {
    const before = settings;
    const next = { ...settings, ...change };
    setSettings(next);
    api.setCryptoSettings(accountId, next).then(setSettings).catch(() => {
      setSettings(before);
      toast({ kind: "error", title: "Não foi possível salvar", description: "Tente de novo em instantes." });
    });
  };

  const email = account.email.toLowerCase();
  const mine = keys.filter((k) => k.mine && (k.emails.includes(email) || k.fingerprint === settings.pgpKey));
  const noKey = settings.pgpKey === null;

  return (
    <>
      <div className="row" style={{ gap: "var(--space-100)" }}>
        <span id="crypto-account" style={{ font: "var(--font-body)", color: "var(--color-text-subtle)" }}>Conta</span>
        <Select labelledBy="crypto-account" value={accountId} options={accounts.map((a) => ({ value: a.id, label: `${a.displayName} · ${a.email}` }))} onChange={(id) => { setSettings(null); setAccountId(id); }} />
      </div>

      <SettingsGroup title="OpenPGP">
        <p className="setting__desc" style={{ margin: 0 }}>A chave que esta conta usa para assinar e para quem quiser escrever para você com criptografia.</p>
        <ChoiceList<string>
          label="Chave OpenPGP desta conta"
          name="crypto-pgp-key"
          value={settings.pgpKey ?? NONE}
          onChange={(v) => save({ pgpKey: v === NONE ? null : v })}
          options={[
            { value: NONE, icon: "x", title: "Nenhuma", description: "Sem OpenPGP nesta conta." },
            ...mine.map((k) => ({
              value: k.fingerprint,
              icon: "key",
              title: (
                <>
                  {keyName(k)} · {k.postQuantum ? "pós-quântica" : k.algorithm}{" "}
                  {k.postQuantum ? <Lozenge appearance="new">Experimental</Lozenge> : <Lozenge appearance="success">Minha</Lozenge>}
                </>
              ),
              description: (
                <>
                  <Fingerprint value={k.fingerprint} compact />
                  {k.postQuantum ? " Só quem também tem uma chave assim consegue ler." : ` ${k.expiresAt === null ? "Sem validade" : `Vale até ${keyDate(k.expiresAt)}`}`}
                </>
              ),
            })),
          ]}
        />
        <div className="row" style={{ gap: "var(--space-100)" }}>
          <Button appearance="subtle" icon="plus" onClick={() => setAdding(true)}>Adicionar chave…</Button>
          <Button appearance="subtle" icon="key" onClick={() => onManageKeys(accountId)}>Gerenciar chaves OpenPGP…</Button>
        </div>
      </SettingsGroup>

      <SettingsGroup title="Ao enviar">
        {noKey && <SettingNote>Escolha ou crie uma chave para esta conta e estas opções passam a valer.</SettingNote>}
        <Setting labelId="crypto-default" label="Mensagens novas" description="O cadeado do composer começa assim; você pode mudar em cada mensagem.">
          <Segmented<"plain" | "encrypted">
            label="Mensagens novas"
            labelledBy="crypto-default"
            value={settings.encryptNew ? "encrypted" : "plain"}
            onChange={(v) => save({ encryptNew: v === "encrypted" })}
            options={[{ value: "plain", label: "Sem criptografia" }, { value: "encrypted", label: "Criptografadas" }]}
          />
        </Setting>
        <Setting labelId="crypto-auto" label="Criptografar sozinho quando todos tiverem chave" description="O cadeado liga enquanto você escolhe os destinatários, se todos tiverem chave aceita.">
          <input type="checkbox" className="toggle" aria-labelledby="crypto-auto" checked={settings.autoEncrypt} onChange={(e) => save({ autoEncrypt: e.target.checked })} />
        </Setting>
        <Setting
          labelId="crypto-sign"
          label="Assinar mensagens sem criptografia"
          description="A assinatura mostra que foi você e que nada mudou. Quem não usa criptografia vê um anexo a mais. Mensagens criptografadas são sempre assinadas."
        >
          <input type="checkbox" className="toggle" aria-labelledby="crypto-sign" checked={settings.signUnencrypted} onChange={(e) => save({ signUnencrypted: e.target.checked })} />
        </Setting>
        <SectionMessage title="Respostas a mensagens criptografadas são criptografadas">
          Responder sem criptografia mostraria o texto citado. Se algum destinatário não tiver chave, o Epistula pergunta antes de enviar.
        </SectionMessage>
      </SettingsGroup>

      <SettingsGroup title="Avançado">
        <Setting labelId="crypto-attach" label="Anexar minha chave pública ao assinar" description="Assim quem recebe consegue responder criptografado.">
          <input type="checkbox" className="toggle" aria-labelledby="crypto-attach" checked={settings.attachKey} onChange={(e) => save({ attachKey: e.target.checked })} />
        </Setting>
        <Setting labelId="crypto-autocrypt" label="Enviar minha chave no cabeçalho (Autocrypt)" description="Programas compatíveis passam a criptografar para você sem ninguém fazer nada.">
          <input type="checkbox" className="toggle" aria-labelledby="crypto-autocrypt" checked={settings.autocrypt} onChange={(e) => save({ autocrypt: e.target.checked })} />
        </Setting>
        <Setting labelId="crypto-subject" label="Criptografar também o assunto" description={'No servidor, o assunto aparece como "...". Quem recebe vê o assunto verdadeiro ao abrir.'}>
          <input type="checkbox" className="toggle" aria-labelledby="crypto-subject" checked={settings.encryptSubject} onChange={(e) => save({ encryptSubject: e.target.checked })} />
        </Setting>
        <Setting labelId="crypto-search" label="Buscar chaves na internet" description="No domínio do destinatário e em keys.openpgp.org. Quem responde a busca fica sabendo com quem você vai falar.">
          <Select labelledBy="crypto-search" value={settings.searchOnline} options={SEARCH_CHOICES} onChange={(v) => save({ searchOnline: v as Settings["searchOnline"] })} />
        </Setting>
        <SettingNote>Rascunhos ficam só neste computador, no banco já criptografado, por isso não há opção para eles.</SettingNote>
      </SettingsGroup>

      {adding && (
        <AddKeyDialog
          account={account}
          onClose={() => setAdding(false)}
          onAdded={(added) => {
            // Uma chave minha importada para esta conta vira a chave dela, se ainda não houver uma.
            const own = added.find((k) => k.mine && k.emails.includes(email));
            if (own && settings.pgpKey === null) save({ pgpKey: own.fingerprint });
            load();
          }}
        />
      )}
    </>
  );
}

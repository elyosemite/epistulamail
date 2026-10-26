// Composição de DS: criptografia.html "Configurações → Criptografia" · .settings__group, .choice-list (L64) com
// .fingerprint--compact (L65), .setting com .toggle, .segmented e .select, .section-message; grupo S/MIME (US14)
import { useCallback, useEffect, useState } from "react";
import { keyDate, keyName } from "../../lib/crypto";
import { api, type Account, type CryptoSettings as Settings, type PgpKey, type SmimeCert } from "../../lib/tauri";
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
import { ImportCertDialog, RequestCertDialog } from "./SmimeDialogs";

const NONE = "none";

const TECHNOLOGY_CHOICES = [
  { value: "auto", label: "Escolher pelas chaves disponíveis" },
  { value: "openpgp", label: "Preferir OpenPGP" },
  { value: "smime", label: "Preferir S/MIME" },
];

const SEARCH_CHOICES = [
  { value: "ask", label: "Perguntar antes" },
  { value: "always", label: "Sempre" },
  { value: "never", label: "Nunca" },
];

interface CryptoSettingsProps {
  accounts: Account[];
  initialAccountId: string;
  onManageKeys: (accountId: string) => void;
  onManageCerts: (accountId: string) => void;
}

/** Criptografia (US13, CA1): a chave de cada conta e como as mensagens saem. */
export function CryptoSettings({ accounts, initialAccountId, onManageKeys, onManageCerts }: CryptoSettingsProps) {
  const [accountId, setAccountId] = useState(accounts.some((a) => a.id === initialAccountId) ? initialAccountId : accounts[0]?.id ?? "");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [keys, setKeys] = useState<PgpKey[]>([]);
  const [certs, setCerts] = useState<SmimeCert[]>([]);
  const [adding, setAdding] = useState(false);
  const [smimeDialog, setSmimeDialog] = useState<"request" | "import" | null>(null);
  const toast = useToast();
  const account = accounts.find((a) => a.id === accountId);

  const load = useCallback(() => {
    if (!accountId) return;
    api.getCryptoSettings(accountId).then(setSettings).catch(() => toast({ kind: "error", title: "Não foi possível carregar a criptografia" }));
    api.listPgpKeys().then(setKeys).catch(() => setKeys([]));
    api.listSmimeCerts().then(setCerts).catch(() => setCerts([]));
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
  const noKey = settings.pgpKey === null && settings.smimeCert === null;
  const myCerts = certs.filter((c) => c.mine && (c.emails.includes(email) || c.fingerprint === settings.smimeCert));
  const certOptions = [
    { value: NONE, label: "Nenhum" },
    ...myCerts.map((c) => ({ value: c.fingerprint, label: `${c.subject} · ${c.issuer} · até ${keyDate(c.notAfter)}` })),
  ];

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

      <SettingsGroup title="S/MIME">
        <Setting labelId="crypto-smime" label="Certificado pessoal" description="Emitido por uma autoridade certificadora. Assina e permite receber mensagens criptografadas.">
          <Select labelledBy="crypto-smime" value={settings.smimeCert ?? NONE} options={certOptions} onChange={(v) => save({ smimeCert: v === NONE ? null : v })} />
        </Setting>
        <div className="row" style={{ gap: "var(--space-100)", flexWrap: "wrap" }}>
          <Button appearance="subtle" icon="certificate" onClick={() => setSmimeDialog("request")}>Pedir um certificado (CSR)…</Button>
          <Button appearance="subtle" icon="upload" onClick={() => setSmimeDialog("import")}>Importar certificado…</Button>
          <Button appearance="subtle" icon="key" onClick={() => onManageCerts(accountId)}>Gerenciar certificados S/MIME…</Button>
        </div>
      </SettingsGroup>

      <SettingsGroup title="Ao enviar">
        {noKey && <SettingNote>Escolha uma chave OpenPGP ou um certificado S/MIME para esta conta e estas opções passam a valer.</SettingNote>}
        <Setting labelId="crypto-default" label="Mensagens novas" description="O cadeado do composer começa assim; você pode mudar em cada mensagem.">
          <Segmented<"plain" | "encrypted">
            label="Mensagens novas"
            labelledBy="crypto-default"
            value={settings.encryptNew ? "encrypted" : "plain"}
            onChange={(v) => save({ encryptNew: v === "encrypted" })}
            options={[{ value: "plain", label: "Sem criptografia" }, { value: "encrypted", label: "Criptografadas" }]}
          />
        </Setting>
        <Setting labelId="crypto-auto" label="Criptografar sozinho quando todos tiverem chave" description="O cadeado liga enquanto você escolhe os destinatários, se todos tiverem chave ou certificado aceitos.">
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
        <Setting labelId="crypto-tech" label="Tecnologia preferida" description="Quando a conta tem as duas e os destinatários aceitam as duas.">
          <Select labelledBy="crypto-tech" value={settings.technology} options={TECHNOLOGY_CHOICES} onChange={(v) => save({ technology: v as Settings["technology"] })} />
        </Setting>
        <Setting labelId="crypto-attach" label="Anexar minha chave pública ao assinar com OpenPGP" description="Assim quem recebe consegue responder criptografado. No S/MIME, o certificado já vai na assinatura.">
          <input type="checkbox" className="toggle" aria-labelledby="crypto-attach" checked={settings.attachKey} onChange={(e) => save({ attachKey: e.target.checked })} />
        </Setting>
        <Setting labelId="crypto-autocrypt" label="Enviar minha chave no cabeçalho (Autocrypt)" description="Programas compatíveis passam a criptografar para você sem ninguém fazer nada.">
          <input type="checkbox" className="toggle" aria-labelledby="crypto-autocrypt" checked={settings.autocrypt} onChange={(e) => save({ autocrypt: e.target.checked })} />
        </Setting>
        <Setting labelId="crypto-subject" label="Criptografar também o assunto" description={'Com OpenPGP, o servidor vê o assunto como "..." e quem recebe vê o verdadeiro ao abrir. Com S/MIME o assunto vai aberto, porque Outlook e Apple Mail não leem o assunto protegido.'}>
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
      {smimeDialog === "request" && (
        <RequestCertDialog account={account} onClose={() => setSmimeDialog(null)} onCreated={() => undefined} onImport={() => setSmimeDialog("import")} />
      )}
      {smimeDialog === "import" && (
        <ImportCertDialog
          onClose={() => setSmimeDialog(null)}
          onImported={(list) => {
            setSmimeDialog(null);
            // O backend já pôs o meu certificado na conta do mesmo e-mail que não tinha um.
            load();
            toast({ kind: "success", title: list.length === 1 ? "Certificado importado" : `${list.length} certificados importados` });
          }}
        />
      )}
    </>
  );
}

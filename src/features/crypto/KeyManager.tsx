// Composição de DS: criptografia.html "Gerenciador de chaves OpenPGP" · .key-manager (__bar, __body), .input--search,
// .segmented, menu "Importar", .key-table (__who, __id, linha com aria-selected) e .key-details (__title) com a aceitação em
// .choice-list (L64-L66); .pem-box (L69) da chave pública; modais.html .modal--warning para excluir
import { useCallback, useEffect, useMemo, useState, type KeyboardEvent } from "react";
import { ACCEPTANCE_CHOICES, keyDate, keyName, keyStatus, shortId } from "../../lib/crypto";
import { keyFileName, pickFile, readKeyFile, saveText } from "../../lib/files";
import { api, type Account, type AppError, type PgpAcceptance, type PgpKey } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { ChoiceList } from "../../ui/ChoiceCard";
import { TextField } from "../../ui/Field";
import { Fingerprint } from "../../ui/Fingerprint";
import { Lozenge } from "../../ui/Lozenge";
import { Menu, anchorBelow, type Anchor } from "../../ui/Menu";
import { Modal } from "../../ui/Modal";
import { PasswordField } from "../../ui/PasswordField";
import { PemBox } from "../../ui/PemBox";
import { SearchField } from "../../ui/SearchField";
import { SectionMessage } from "../../ui/SectionMessage";
import { Segmented } from "../../ui/Segmented";
import { Spinner } from "../../ui/Spinner";
import { useToast } from "../../ui/Toast";
import { AddKeyDialog, BackupDialog } from "./AddKeyDialog";

type Filter = "all" | "mine" | "contacts";

const ORIGIN: Record<PgpKey["origin"], string> = {
  generated: "Criada neste computador",
  imported: "Importada",
  autocrypt: "Cabeçalho Autocrypt de uma mensagem",
  attachment: "Anexo de uma mensagem",
  search: "Busca na internet",
};

const KEY_ACCEPT = ".asc,.gpg,.key,.pgp,application/pgp-keys";

type Dialog =
  | { kind: "add" }
  | { kind: "passphrase"; data: string; binary: boolean }
  | { kind: "search" }
  | { kind: "public"; key: PgpKey; armored: string }
  | { kind: "backup"; key: PgpKey }
  | { kind: "delete"; key: PgpKey }
  | null;

function matches(key: PgpKey, query: string): boolean {
  const q = query.trim().toLowerCase().replace(/\s+/g, "");
  if (!q) return true;
  return key.fingerprint.toLowerCase().includes(q) || [...key.userIds, ...key.emails].some((t) => t.toLowerCase().replace(/\s+/g, "").includes(q));
}

/** Gerenciador de chaves OpenPGP (CA3): todas as chaves deste computador, com busca, importação e aceitação. */
export function KeyManager({ account, initialKey }: { account: Account; initialKey?: string | null }) {
  const [keys, setKeys] = useState<PgpKey[] | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [selected, setSelected] = useState<string | null>(initialKey ?? null);
  const [importMenu, setImportMenu] = useState<Anchor | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const toast = useToast();

  const load = useCallback(() => {
    api.listPgpKeys().then(setKeys).catch(() => { setKeys([]); toast({ kind: "error", title: "Não foi possível carregar as chaves" }); });
  }, [toast]);
  useEffect(load, [load]);

  const shown = useMemo(
    () => (keys ?? []).filter((k) => (filter === "mine" ? k.mine : filter === "contacts" ? !k.mine : true) && matches(k, query)),
    [keys, filter, query],
  );
  const current = shown.find((k) => k.fingerprint === selected) ?? shown[0] ?? null;

  const imported = (list: PgpKey[]) => {
    load();
    if (list.length === 0) toast({ kind: "warning", title: "Nenhuma chave encontrada" });
    else {
      setSelected(list[0].fingerprint);
      toast({ kind: "success", title: list.length === 1 ? "Chave importada" : `${list.length} chaves importadas` });
    }
  };

  const importData = async (data: string, binary: boolean) => {
    try {
      imported(await api.importPgpKeys(data, binary));
    } catch (e) {
      if ((e as AppError).kind === "WrongPassword") setDialog({ kind: "passphrase", data, binary });
      else toast({ kind: "error", title: "Não deu para ler uma chave OpenPGP aí" });
    }
  };

  const fromFile = async () => {
    const file = await pickFile(KEY_ACCEPT);
    if (!file) return;
    const { data, binary } = await readKeyFile(file);
    await importData(data, binary);
  };

  const fromClipboard = async () => {
    const text = await navigator.clipboard.readText().catch(() => "");
    if (!text.includes("-----BEGIN PGP")) {
      toast({ kind: "warning", title: "A área de transferência não tem uma chave OpenPGP" });
      return;
    }
    await importData(text, false);
  };

  const exportPublic = async (key: PgpKey) => {
    try {
      setDialog({ kind: "public", key, armored: await api.exportPgpPublic(key.fingerprint) });
    } catch {
      toast({ kind: "error", title: "Não foi possível exportar a chave" });
    }
  };

  const setAcceptance = (key: PgpKey, acceptance: PgpAcceptance) => {
    setKeys((list) => list?.map((k) => (k.fingerprint === key.fingerprint ? { ...k, acceptance } : k)) ?? list);
    api.setPgpAcceptance(key.fingerprint, acceptance).catch(() => { toast({ kind: "error", title: "Não foi possível salvar" }); load(); });
  };

  const onTableKey = (e: KeyboardEvent<HTMLTableSectionElement>) => {
    const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (!step || shown.length === 0) return;
    e.preventDefault();
    const i = current ? shown.indexOf(current) : -1;
    const next = shown[Math.min(shown.length - 1, Math.max(0, i + step))];
    setSelected(next.fingerprint);
    e.currentTarget.querySelector<HTMLElement>(`[data-fp="${next.fingerprint}"]`)?.focus();
  };

  if (!keys) return <Spinner label="Carregando as chaves" />;

  return (
    <div className="key-manager" style={{ padding: "var(--space-200)", overflow: "hidden", flex: 1 }}>
      <div className="key-manager__bar">
        <SearchField value={query} placeholder="Buscar por nome, e-mail ou impressão digital" label="Buscar chaves" onChange={setQuery} onEscape={() => setQuery("")} />
        <Segmented<Filter>
          label="Mostrar"
          value={filter}
          onChange={setFilter}
          options={[{ value: "all", label: "Todas" }, { value: "mine", label: "Minhas" }, { value: "contacts", label: "De contatos" }]}
        />
        <Button appearance="subtle" icon="upload" aria-haspopup="menu" onClick={(e) => setImportMenu(anchorBelow(e.currentTarget))}>Importar</Button>
        <Button appearance="primary" icon="plus" onClick={() => setDialog({ kind: "add" })}>Nova chave</Button>
      </div>
      <div className="key-manager__body">
        <div style={{ overflow: "auto" }}>
          {shown.length === 0 ? (
            <p className="setting__desc">{keys.length === 0 ? "Nenhuma chave ainda. Crie a sua ou importe as de seus contatos." : "Nenhuma chave com essa busca."}</p>
          ) : (
            <table className="key-table">
              <thead><tr><th>Nome e e-mail</th><th>ID</th><th>Validade</th><th>Situação</th></tr></thead>
              <tbody onKeyDown={onTableKey}>
                {shown.map((k) => {
                  const status = keyStatus(k);
                  const isCurrent = k === current;
                  return (
                    <tr
                      key={k.fingerprint}
                      data-fp={k.fingerprint}
                      aria-selected={isCurrent}
                      tabIndex={isCurrent ? 0 : -1}
                      style={{ cursor: "default" }}
                      onClick={() => setSelected(k.fingerprint)}
                    >
                      <td><span className="key-table__who"><b>{keyName(k)}</b>{k.emails[0]}</span></td>
                      <td className="key-table__id">{shortId(k.fingerprint)}</td>
                      <td>{keyDate(k.expiresAt)}</td>
                      <td>
                        <Lozenge appearance={status.tone}>{status.text}</Lozenge>
                        {k.postQuantum && <> <Lozenge appearance="new">Pós-quântica</Lozenge></>}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
        {current && (
          <KeyDetails
            key={current.fingerprint}
            pgpKey={current}
            onAcceptance={(a) => setAcceptance(current, a)}
            onExport={() => void exportPublic(current)}
            onBackup={() => setDialog({ kind: "backup", key: current })}
            onDelete={() => setDialog({ kind: "delete", key: current })}
          />
        )}
      </div>

      {importMenu && (
        <Menu
          label="Importar"
          anchor={importMenu}
          onClose={() => setImportMenu(null)}
          entries={[
            { label: "De um arquivo…", icon: "file", onSelect: () => void fromFile() },
            { label: "Da área de transferência", icon: "copy", onSelect: () => void fromClipboard() },
            { label: "Buscar na internet…", icon: "search", onSelect: () => setDialog({ kind: "search" }) },
          ]}
        />
      )}
      {dialog?.kind === "add" && <AddKeyDialog account={account} onClose={() => setDialog(null)} onAdded={(list) => { load(); if (list[0]) setSelected(list[0].fingerprint); }} />}
      {dialog?.kind === "passphrase" && (
        <PassphraseDialog
          onClose={() => setDialog(null)}
          onSubmit={async (passphrase) => {
            imported(await api.importPgpKeys(dialog.data, dialog.binary, passphrase));
            setDialog(null);
          }}
        />
      )}
      {dialog?.kind === "search" && <SearchKeyDialog onClose={() => setDialog(null)} onFound={(list) => { setDialog(null); imported(list); }} />}
      {dialog?.kind === "public" && (
        <Modal
          title="Chave pública"
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button appearance="subtle" onClick={() => setDialog(null)}>Fechar</Button>
              <Button appearance="primary" icon="download" onClick={() => { saveText(keyFileName(keyName(dialog.key), "publica"), dialog.armored); setDialog(null); }}>Salvar arquivo</Button>
            </>
          }
        >
          <div style={{ display: "grid", gap: "var(--space-150)" }}>
            <p className="setting__desc" style={{ margin: 0 }}>Pode mandar para qualquer pessoa: com ela, escrevem para você com criptografia e conferem a sua assinatura.</p>
            <PemBox text={dialog.armored} label={`Chave pública de ${keyName(dialog.key)}`} />
          </div>
        </Modal>
      )}
      {dialog?.kind === "backup" && <BackupDialog pgpKey={dialog.key} onClose={() => setDialog(null)} />}
      {dialog?.kind === "delete" && (
        <Modal
          title={dialog.key.mine ? "Excluir a sua chave?" : "Excluir a chave?"}
          appearance="warning"
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button appearance="subtle" onClick={() => setDialog(null)}>Cancelar</Button>
              <Button
                appearance="warning"
                onClick={() => {
                  const fp = dialog.key.fingerprint;
                  setDialog(null);
                  api.deletePgpKey(fp).then(() => { setSelected(null); load(); toast({ kind: "success", title: "Chave excluída" }); }).catch(() => toast({ kind: "error", title: "Não foi possível excluir" }));
                }}
              >
                Excluir
              </Button>
            </>
          }
        >
          {dialog.key.mine ? (
            <p>Sem ela, as mensagens criptografadas para você não abrem mais neste computador. Só exclua se tiver um backup. As contas que a usam ficam sem OpenPGP.</p>
          ) : (
            <p>As mensagens para {dialog.key.emails[0] ?? keyName(dialog.key)} deixam de poder ir criptografadas até a chave voltar.</p>
          )}
        </Modal>
      )}
    </div>
  );
}

function KeyDetails({ pgpKey: k, onAcceptance, onExport, onBackup, onDelete }: { pgpKey: PgpKey; onAcceptance: (a: PgpAcceptance) => void; onExport: () => void; onBackup: () => void; onDelete: () => void }) {
  const titleId = `kd-${k.fingerprint}`;
  const accId = `kd-acc-${k.fingerprint}`;
  const email = k.emails[0] ?? keyName(k);
  return (
    <aside className="key-details" aria-labelledby={titleId}>
      <h3 className="key-details__title" id={titleId}>{keyName(k)}{k.mine && <> <Lozenge appearance="success">Minha</Lozenge></>}</h3>
      <div className="field"><span className="field__label">Impressão digital</span><Fingerprint value={k.fingerprint} /></div>
      <dl>
        <dt>E-mail</dt><dd>{k.emails.join(", ") || "Nenhum"}</dd>
        <dt>Tipo</dt><dd>{k.algorithm}{k.postQuantum ? " (pós-quântica)" : ""}</dd>
        <dt>Criada</dt><dd>{keyDate(k.createdAt)}</dd>
        <dt>Vale até</dt><dd>{keyDate(k.expiresAt)}</dd>
        <dt>Chegou por</dt><dd>{ORIGIN[k.origin]}</dd>
      </dl>
      {!k.mine && (
        <div style={{ display: "grid", gap: "var(--space-075)" }}>
          <span className="field__label" id={accId}>Você aceita esta chave para {email}?</span>
          <ChoiceList<PgpAcceptance>
            label={`Aceitação da chave de ${email}`}
            name={accId}
            value={k.acceptance}
            onChange={onAcceptance}
            options={ACCEPTANCE_CHOICES.map((c) => ({ value: c.value, title: c.label }))}
          />
        </div>
      )}
      <div className="row" style={{ gap: "var(--space-075)", flexWrap: "wrap" }}>
        <Button appearance="subtle" compact icon="download" onClick={onExport}>{k.mine ? "Exportar pública" : "Exportar"}</Button>
        {k.mine && <Button appearance="subtle" compact icon="shield" onClick={onBackup}>Backup da secreta…</Button>}
        <Button appearance="subtle" compact icon="trash" onClick={onDelete}>Excluir</Button>
      </div>
    </aside>
  );
}

function PassphraseDialog({ onClose, onSubmit }: { onClose: () => void; onSubmit: (passphrase: string) => Promise<void> }) {
  const [passphrase, setPassphrase] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | undefined>();
  const submit = async () => {
    setBusy(true);
    setError(undefined);
    try {
      await onSubmit(passphrase);
    } catch (e) {
      setError((e as AppError).kind === "WrongPassword" ? "A senha não confere." : "Não foi possível importar.");
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      title="A chave secreta tem senha"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" loading={busy} disabled={!passphrase} onClick={() => void submit()}>Importar</Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); if (passphrase) void submit(); }}>
        <PasswordField label="Senha da chave" value={passphrase} autoComplete="off" error={error} help="Depois de importada, a chave fica protegida pela criptografia do Epistula." onChange={(e) => setPassphrase(e.target.value)} />
      </form>
    </Modal>
  );
}

/** "Buscar na internet…": diz antes onde vai procurar; clicar em Buscar é o consentimento (D5). */
export function SearchKeyDialog({ initialEmail = "", onClose, onFound }: { initialEmail?: string; onClose: () => void; onFound: (keys: PgpKey[]) => void }) {
  const [email, setEmail] = useState(initialEmail);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const domain = email.includes("@") ? email.split("@")[1] : "do e-mail";
  const search = async () => {
    setBusy(true);
    setError(null);
    try {
      const keys = await api.searchPgpKey(email.trim());
      if (keys.length === 0) setError(`Nenhuma chave publicada para ${email.trim()}.`);
      else onFound(keys);
    } catch {
      setError("A busca não respondeu. Confira a conexão e tente de novo.");
    } finally {
      setBusy(false);
    }
  };
  const valid = /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim());
  return (
    <Modal
      title="Buscar chave na internet"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" icon="search" loading={busy} disabled={!valid} onClick={() => void search()}>Buscar</Button>
        </>
      }
    >
      <form style={{ display: "grid", gap: "var(--space-150)" }} onSubmit={(e) => { e.preventDefault(); if (valid) void search(); }}>
        <TextField label="E-mail" type="email" value={email} autoComplete="off" onChange={(e) => setEmail(e.target.value)} />
        <p className="setting__desc" style={{ margin: 0 }}>
          O Epistula pergunta ao domínio {domain} e a keys.openpgp.org. Quem responde a busca fica sabendo que alguém procurou este endereço.
          A chave chega como "ainda não aceita".
        </p>
        {error && <SectionMessage appearance="warning" title={error} />}
      </form>
    </Modal>
  );
}

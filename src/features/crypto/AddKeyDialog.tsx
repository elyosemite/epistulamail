// Composição de DS: criptografia.html "Adicionar chave OpenPGP" · .modal com .wizard-steps, .choice-list (L64), .field,
// .fingerprint (L65) e .section-message--warning do backup; PasswordField (campos.html "Senha")
import { useState } from "react";
import { KEY_KINDS, keyName } from "../../lib/crypto";
import { keyFileName, pickFile, readKeyFile, saveText } from "../../lib/files";
import { api, type Account, type AppError, type PgpKey, type PgpKeyKind } from "../../lib/tauri";
import { Button, IconButton } from "../../ui/Button";
import { ChoiceList } from "../../ui/ChoiceCard";
import { TextField } from "../../ui/Field";
import { Fingerprint } from "../../ui/Fingerprint";
import { Lozenge } from "../../ui/Lozenge";
import { Modal } from "../../ui/Modal";
import { PasswordField } from "../../ui/PasswordField";
import { SectionMessage } from "../../ui/SectionMessage";

type Path = "create" | "import" | "restore";
type Step = { kind: "choose" } | { kind: "create" } | { kind: "file"; path: "import" | "restore" } | { kind: "done"; keys: PgpKey[]; created: boolean };

const KEY_ACCEPT = ".asc,.gpg,.key,.pgp,application/pgp-keys";

/** "Adicionar chave…" (CA2): criar, importar ou restaurar, em três passos. */
export function AddKeyDialog({ account, onClose, onAdded }: { account: Account; onClose: () => void; onAdded: (keys: PgpKey[]) => void }) {
  const [step, setStep] = useState<Step>({ kind: "choose" });
  const [path, setPath] = useState<Path>("create");
  const [name, setName] = useState(account.displayName);
  const [kind, setKind] = useState<PgpKeyKind>("curve25519");
  const [file, setFile] = useState<File | null>(null);
  const [passphrase, setPassphrase] = useState("");
  const [needsPassphrase, setNeedsPassphrase] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [backup, setBackup] = useState<PgpKey | null>(null);

  const finish = (keys: PgpKey[], created: boolean) => {
    onAdded(keys);
    setStep({ kind: "done", keys, created });
  };

  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      finish([await api.generatePgpKey(account.id, kind, name.trim())], true);
    } catch {
      setError("Não foi possível criar a chave. Tente de novo.");
    } finally {
      setBusy(false);
    }
  };

  const importFile = async () => {
    if (!file) return;
    setBusy(true);
    setError(null);
    try {
      const { data, binary } = await readKeyFile(file);
      const keys = await api.importPgpKeys(data, binary, passphrase || null);
      if (keys.length === 0) setError("O arquivo não tem nenhuma chave OpenPGP.");
      else finish(keys, false);
    } catch (e) {
      const kind = (e as AppError).kind;
      if (kind === "WrongPassword") {
        setError(needsPassphrase && passphrase ? "A senha não confere." : null);
        setNeedsPassphrase(true);
      } else setError("O arquivo não tem uma chave OpenPGP que dê para ler.");
    } finally {
      setBusy(false);
    }
  };

  if (backup) return <BackupDialog pgpKey={backup} onClose={() => setBackup(null)} onSaved={onClose} />;

  if (step.kind === "choose") {
    return (
      <Modal
        title="Adicionar chave OpenPGP"
        onClose={onClose}
        footer={
          <>
            <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
            <Button appearance="primary" onClick={() => setStep(path === "create" ? { kind: "create" } : { kind: "file", path })}>Continuar</Button>
          </>
        }
      >
        <p className="wizard-steps">Passo 1 de 3</p>
        <ChoiceList<Path>
          label="Como adicionar"
          name="add-key-path"
          value={path}
          onChange={setPath}
          options={[
            { value: "create", icon: "sparkle", title: "Criar uma chave nova", description: `Para ${account.email}. Leva alguns segundos.` },
            { value: "import", icon: "upload", title: "Importar uma chave que já tenho", description: "Arquivo .asc, .gpg ou .key, de outro programa ou computador." },
            { value: "restore", icon: "download", title: "Restaurar um backup do EpistulaMail", description: "O arquivo que você salvou ao criar a chave." },
          ]}
        />
      </Modal>
    );
  }

  if (step.kind === "create") {
    return (
      <Modal
        title="Criar uma chave nova"
        splitFooter
        onClose={onClose}
        footer={
          <>
            <Button appearance="subtle" disabled={busy} onClick={() => setStep({ kind: "choose" })}>Voltar</Button>
            <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
            <Button appearance="primary" loading={busy} disabled={!name.trim()} onClick={() => void create()}>Criar chave</Button>
          </>
        }
      >
        <div style={{ display: "grid", gap: "var(--space-200)" }}>
          <p className="wizard-steps" style={{ margin: 0 }}>Passo 2 de 3</p>
          <TextField label="Nome" value={name} maxLength={120} onChange={(e) => setName(e.target.value)} />
          <div className="field">
            <span className="field__label">E-mail</span>
            <span style={{ font: "var(--font-body)" }}>{account.email}</span>
          </div>
          <div style={{ display: "grid", gap: "var(--space-100)" }}>
            <span className="field__label">Tipo</span>
            <ChoiceList<PgpKeyKind>
              label="Tipo da chave"
              name="add-key-kind"
              value={kind}
              onChange={setKind}
              options={KEY_KINDS.map((k) => ({
                value: k.value,
                icon: k.icon,
                description: k.description,
                title: <>{k.title}{k.badge && <> <Lozenge appearance={k.badge.tone}>{k.badge.text}</Lozenge></>}</>,
              }))}
            />
          </div>
          {error && <SectionMessage appearance="error" title={error} />}
        </div>
      </Modal>
    );
  }

  if (step.kind === "file") {
    const restore = step.path === "restore";
    return (
      <Modal
        title={restore ? "Restaurar um backup" : "Importar uma chave"}
        splitFooter
        onClose={onClose}
        footer={
          <>
            <Button appearance="subtle" disabled={busy} onClick={() => { setStep({ kind: "choose" }); setError(null); }}>Voltar</Button>
            <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
            <Button appearance="primary" loading={busy} disabled={!file || (needsPassphrase && !passphrase)} onClick={() => void importFile()}>
              {restore ? "Restaurar" : "Importar"}
            </Button>
          </>
        }
      >
        <div style={{ display: "grid", gap: "var(--space-200)" }}>
          <p className="wizard-steps" style={{ margin: 0 }}>Passo 2 de 3</p>
          <div className="field">
            <span className="field__label">Arquivo</span>
            <div className="row" style={{ gap: "var(--space-100)" }}>
              <Button
                appearance="subtle"
                icon="file"
                onClick={() => void pickFile(KEY_ACCEPT).then((f) => { if (f) { setFile(f); setError(null); setNeedsPassphrase(restore); setPassphrase(""); } })}
              >
                Escolher arquivo…
              </Button>
              <span style={{ font: "var(--font-body)", color: file ? "var(--color-text)" : "var(--color-text-subtlest)", overflowWrap: "anywhere" }}>{file ? file.name : "Nenhum arquivo escolhido"}</span>
            </div>
          </div>
          {(needsPassphrase || restore) && file && (
            <PasswordField
              label={restore ? "Senha do backup" : "Senha da chave"}
              value={passphrase}
              autoComplete="off"
              help="A senha que protege a chave secreta no arquivo."
              onChange={(e) => setPassphrase(e.target.value)}
            />
          )}
          {error && <SectionMessage appearance="error" title={error} />}
        </div>
      </Modal>
    );
  }

  const first = step.keys[0];
  if (!step.created) {
    return (
      <Modal title={step.keys.length === 1 ? "Chave adicionada" : `${step.keys.length} chaves adicionadas`} onClose={onClose} footer={<Button appearance="primary" onClick={onClose}>Pronto</Button>}>
        <div style={{ display: "grid", gap: "var(--space-150)" }}>
          <p className="wizard-steps" style={{ margin: 0 }}>Passo 3 de 3</p>
          {step.keys.map((k) => (
            <div key={k.fingerprint} className="field">
              <span className="field__label">{keyName(k)}{k.mine && <> <Lozenge appearance="success">Minha</Lozenge></>}</span>
              <Fingerprint value={k.fingerprint} />
            </div>
          ))}
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      title="Sua chave está pronta"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" onClick={onClose}>Depois</Button>
          <Button appearance="primary" icon="download" onClick={() => setBackup(first)}>Salvar backup…</Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <p className="wizard-steps" style={{ margin: 0 }}>Passo 3 de 3</p>
        <div className="field">
          <span className="field__label">Impressão digital</span>
          <div className="row" style={{ gap: "var(--space-100)", flexWrap: "nowrap", alignItems: "center" }}>
            <Fingerprint value={first.fingerprint} />
            <IconButton icon="copy" label="Copiar a impressão digital" data-tip="Copiar" compact onClick={() => void navigator.clipboard.writeText(first.fingerprint).catch(() => undefined)} />
          </div>
          <span className="field__help">Quem quiser ter certeza de que a chave é sua confere estes números com você, por outro meio.</span>
        </div>
        <SectionMessage appearance="warning" title="Faça um backup agora">
          Sem a chave, as mensagens criptografadas para você não abrem nunca mais, nem em outro computador. O backup sai protegido por uma senha que você escolhe.
        </SectionMessage>
      </div>
    </Modal>
  );
}

/** "Backup da secreta…": a chave secreta protegida por uma senha nova, salva como arquivo. */
export function BackupDialog({ pgpKey, onClose, onSaved }: { pgpKey: PgpKey; onClose: () => void; onSaved?: () => void }) {
  const [passphrase, setPassphrase] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const short = passphrase.length > 0 && passphrase.length < 8;
  const mismatch = confirm.length > 0 && confirm !== passphrase;

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      const armored = await api.backupPgpSecret(pgpKey.fingerprint, passphrase);
      saveText(keyFileName(keyName(pgpKey), "backup-secreto"), armored);
      (onSaved ?? onClose)();
    } catch {
      setError("Não foi possível fazer o backup. Tente de novo.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Backup da chave secreta"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" icon="download" loading={busy} disabled={passphrase.length < 8 || confirm !== passphrase} onClick={() => void save()}>Salvar backup</Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <p className="setting__desc" style={{ margin: 0 }}>
          O arquivo vai para a pasta Downloads. Guarde-o fora deste computador; sem a senha, ninguém usa a chave, nem você.
        </p>
        <PasswordField label="Senha do backup" value={passphrase} autoComplete="new-password" error={short ? "Use pelo menos 8 caracteres." : undefined} onChange={(e) => setPassphrase(e.target.value)} />
        <PasswordField label="Repita a senha" value={confirm} autoComplete="new-password" error={mismatch ? "As senhas não são iguais." : undefined} onChange={(e) => setConfirm(e.target.value)} />
        {error && <SectionMessage appearance="error" title={error} />}
      </div>
    </Modal>
  );
}

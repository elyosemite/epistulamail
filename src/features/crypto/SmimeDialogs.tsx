// Composição de DS: criptografia.html "S/MIME: pedir certificado e gerenciar" · .modal com .field, .select, .pem-box (L69) e
// .modal__foot--split; PasswordField (campos.html "Senha") para o .p12
import { useState } from "react";
import { CERT_KINDS, COUNTRIES } from "../../lib/crypto";
import { keyFileName, pickFile, readKeyFile, saveBase64, saveText } from "../../lib/files";
import { api, type Account, type AppError, type SmimeCert, type SmimeCertKind, type SmimeRequest } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { TextField } from "../../ui/Field";
import { Modal } from "../../ui/Modal";
import { PasswordField } from "../../ui/PasswordField";
import { PemBox } from "../../ui/PemBox";
import { SectionMessage } from "../../ui/SectionMessage";
import { Select } from "../../ui/Select";

export const CERT_ACCEPT = ".cer,.crt,.pem,.der,.p7b,.p7c,.p12,.pfx,application/pkix-cert,application/x-pkcs12";

export interface RequestDefaults {
  name: string;
  organization: string | null;
  country: string | null;
  kind: SmimeCertKind;
}

/** "Pedir um certificado (CSR)…" (CA1): os dados, depois o pedido pronto para levar à autoridade certificadora. */
export function RequestCertDialog({ account, defaults, onClose, onCreated, onImport }: {
  account: Account;
  defaults?: RequestDefaults;
  onClose: () => void;
  onCreated: (request: SmimeRequest) => void;
  /** "Importar certificado emitido…" no fim. */
  onImport: () => void;
}) {
  const [name, setName] = useState(defaults?.name ?? account.displayName);
  const [organization, setOrganization] = useState(defaults?.organization ?? "");
  const [country, setCountry] = useState(defaults?.country ?? "BR");
  const [kind, setKind] = useState<SmimeCertKind>(defaults?.kind ?? "rsa3072");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<SmimeRequest | null>(null);

  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      const request = await api.createSmimeRequest(account.id, kind, name.trim(), organization.trim() || null, country || null);
      onCreated(request);
      setDone(request);
    } catch {
      setError("Não foi possível gerar o pedido. Tente de novo.");
    } finally {
      setBusy(false);
    }
  };

  if (done) {
    return (
      <Modal
        title="Pedido pronto"
        splitFooter
        onClose={onClose}
        footer={
          <>
            <Button appearance="subtle" icon="download" onClick={() => saveText(keyFileName(done.name, "pedido", "csr"), done.csrPem, "application/pkcs10")}>Salvar .csr…</Button>
            <Button appearance="subtle" onClick={onClose}>Fechar</Button>
            <Button appearance="primary" onClick={onImport}>Importar certificado emitido…</Button>
          </>
        }
      >
        <div style={{ display: "grid", gap: "var(--space-150)" }}>
          <p style={{ margin: 0 }}>
            Envie este pedido à autoridade certificadora (no site dela, na parte de certificados de e-mail). Quando ela devolver o certificado, use <b>Importar certificado emitido</b>.
          </p>
          <PemBox text={done.csrPem} label="Pedido de certificado" />
          <p className="field__help" style={{ margin: 0 }}>A chave privada ficou neste computador, criptografada. Ela não sai daqui.</p>
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      title="Pedir um certificado S/MIME"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" loading={busy} disabled={!name.trim()} onClick={() => void create()}>Gerar pedido</Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <div className="field">
          <span className="field__label">E-mail</span>
          <span style={{ font: "var(--font-body)" }}>{account.email}</span>
        </div>
        <TextField label="Nome" value={name} maxLength={64} onChange={(e) => setName(e.target.value)} />
        <TextField label="Organização (opcional)" value={organization} maxLength={64} onChange={(e) => setOrganization(e.target.value)} />
        <div className="field">
          <span className="field__label" id="csr-country">País</span>
          <Select labelledBy="csr-country" value={country} options={COUNTRIES} onChange={setCountry} />
        </div>
        <div className="field">
          <span className="field__label" id="csr-kind">Algoritmo</span>
          <Select labelledBy="csr-kind" value={kind} options={CERT_KINDS} onChange={(v) => setKind(v as SmimeCertKind)} />
        </div>
        {error && <SectionMessage appearance="error" title={error} />}
      </div>
    </Modal>
  );
}

function importError(e: unknown): string {
  switch ((e as AppError).kind) {
    case "CertificateEmailMismatch":
      return "O certificado foi emitido para outro e-mail, não para o do pedido.";
    case "WrongPassword":
      return "A senha não confere.";
    default:
      return "O arquivo não tem um certificado que dê para ler.";
  }
}

/** "Importar certificado…": o emitido para um pedido, um .p12 ou .pfx com a senha, ou o certificado de um contato. */
export function ImportCertDialog({ onClose, onImported }: { onClose: () => void; onImported: (certs: SmimeCert[]) => void }) {
  const [file, setFile] = useState<File | null>(null);
  const [password, setPassword] = useState("");
  const [needsPassword, setNeedsPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const choose = async () => {
    const f = await pickFile(CERT_ACCEPT);
    if (!f) return;
    setFile(f);
    setError(null);
    setPassword("");
    setNeedsPassword(/\.(p12|pfx)$/i.test(f.name));
  };

  const run = async () => {
    if (!file) return;
    setBusy(true);
    setError(null);
    try {
      const { data, binary } = await readKeyFile(file);
      onImported(await api.importSmime(data, binary, password || null));
    } catch (e) {
      if ((e as AppError).kind === "WrongPassword" && !needsPassword) setNeedsPassword(true);
      else setError(importError(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Importar certificado"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" loading={busy} disabled={!file || (needsPassword && !password)} onClick={() => void run()}>Importar</Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <p className="setting__desc" style={{ margin: 0 }}>
          O certificado que a autoridade devolveu (.cer, .crt, .pem ou .p7b), um certificado com a chave (.p12 ou .pfx) ou o certificado de um contato.
        </p>
        <div className="field">
          <span className="field__label">Arquivo</span>
          <div className="row" style={{ gap: "var(--space-100)" }}>
            <Button appearance="subtle" icon="file" onClick={() => void choose()}>Escolher arquivo…</Button>
            <span style={{ font: "var(--font-body)", color: file ? "var(--color-text)" : "var(--color-text-subtlest)", overflowWrap: "anywhere" }}>{file ? file.name : "Nenhum arquivo escolhido"}</span>
          </div>
        </div>
        {needsPassword && file && (
          <PasswordField label="Senha do arquivo" value={password} autoComplete="off" help="A senha que protege a chave dentro do .p12." onChange={(e) => setPassword(e.target.value)} />
        )}
        {error && <SectionMessage appearance="error" title={error} />}
      </div>
    </Modal>
  );
}

/** "Backup .p12…": o meu certificado com a chave e a cadeia, protegidos por uma senha nova. */
export function Pkcs12Dialog({ cert, onClose }: { cert: SmimeCert; onClose: () => void }) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const short = password.length > 0 && password.length < 8;
  const mismatch = confirm.length > 0 && confirm !== password;

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      saveBase64(keyFileName(cert.subject, "certificado", "p12"), await api.exportSmimePkcs12(cert.fingerprint, password));
      onClose();
    } catch {
      setError("Não foi possível exportar. Tente de novo.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Backup do certificado com a chave"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" disabled={busy} onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" icon="download" loading={busy} disabled={password.length < 8 || confirm !== password} onClick={() => void save()}>Salvar .p12</Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <p className="setting__desc" style={{ margin: 0 }}>
          O arquivo vai para a pasta Downloads e abre no Outlook, no Thunderbird ou em outro computador. Guarde-o fora deste computador; sem a senha, ninguém usa a chave.
        </p>
        <PasswordField label="Senha do arquivo" value={password} autoComplete="new-password" error={short ? "Use pelo menos 8 caracteres." : undefined} onChange={(e) => setPassword(e.target.value)} />
        <PasswordField label="Repita a senha" value={confirm} autoComplete="new-password" error={mismatch ? "As senhas não são iguais." : undefined} onChange={(e) => setConfirm(e.target.value)} />
        {error && <SectionMessage appearance="error" title={error} />}
      </div>
    </Modal>
  );
}

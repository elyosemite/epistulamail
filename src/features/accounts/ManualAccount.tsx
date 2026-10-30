// Composição de DS: campos.html (.field, .input, .input--password, .select), mensagens.html (.section-message),
// botoes.html (.btn) e selecao.html (.choice, .checkbox).
import { useState } from "react";
import { api, isAppError, type AppError, type Connected, type ManualTest } from "../../lib/tauri";
import { INITIAL_MANUAL_FORM, manualAccountInput, validateManualForm, type ManualFieldErrors, type ManualFormState } from "../../lib/manualAccount";
import { Button } from "../../ui/Button";
import { TextField } from "../../ui/Field";
import { PasswordField } from "../../ui/PasswordField";
import { SectionMessage } from "../../ui/SectionMessage";
import { Select, type SelectOption } from "../../ui/Select";

const SECURITY_OPTIONS: SelectOption[] = [
  { value: "tls", label: "SSL/TLS" },
  { value: "starttls", label: "STARTTLS" },
  { value: "auto", label: "Detectar automaticamente (em breve)", disabled: true },
];

const AUTH_OPTIONS: SelectOption[] = [
  { value: "password", label: "Senha normal" },
  { value: "encrypted", label: "Senha criptografada (em breve)", disabled: true },
  { value: "gssapi", label: "Kerberos / GSSAPI (em breve)", disabled: true },
  { value: "ntlm", label: "NTLM (em breve)", disabled: true },
  { value: "oauth2", label: "OAuth2 (em breve)", disabled: true },
];

const UNKNOWN_ERROR: AppError = { kind: "AuthFailed", message: "Algo deu errado. Tente de novo." };

interface ManualAccountProps {
  onConnected: (result: Connected) => void;
  onBack: () => void;
}

export function ManualAccount({ onConnected, onBack }: ManualAccountProps) {
  const [form, setForm] = useState<ManualFormState>(INITIAL_MANUAL_FORM);
  const [errors, setErrors] = useState<ManualFieldErrors>({});
  const [test, setTest] = useState<ManualTest | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<AppError | null>(null);

  const update = <K extends keyof ManualFormState>(key: K, value: ManualFormState[K]) => {
    setForm((current) => ({ ...current, [key]: value }));
    setTest(null);
    setError(null);
    setErrors((current) => ({ ...current, [key]: undefined }));
  };

  const runTest = async () => {
    const found = validateManualForm(form);
    setErrors(found);
    if (Object.keys(found).length > 0) {
      setTest(null);
      return;
    }
    setTesting(true);
    setTest(null);
    setError(null);
    try {
      setTest(await api.testManualAccount(manualAccountInput(form)));
    } catch (e) {
      setError(isAppError(e) ? e : UNKNOWN_ERROR);
    } finally {
      setTesting(false);
    }
  };

  const submit = async () => {
    const found = validateManualForm(form);
    setErrors(found);
    if (Object.keys(found).length > 0) {
      setTest(null);
      return;
    }
    setSaving(true);
    setError(null);
    try {
      onConnected(await api.connectManualAccount(manualAccountInput(form)));
    } catch (e) {
      setError(isAppError(e) ? e : UNKNOWN_ERROR);
      setSaving(false);
    }
  };

  const testedOk = test?.incoming.ok === true && test?.outgoing.ok === true;

  return (
    <form className="settings__group" onSubmit={(e) => { e.preventDefault(); if (testedOk) void submit(); }}>
      <div>
        <h2 className="settings__title">Conectar uma conta</h2>
        <p className="setting__desc">
          Informe os servidores de entrada e de saída. Teste os dois antes de conectar; a senha fica no cofre do sistema.
        </p>
      </div>

      {error && <SectionMessage appearance="error" title="Não foi possível conectar">{error.message}</SectionMessage>}

      <div className="settings__group">
          <h3 className="settings__group-title">Entrada</h3>
          <TextField label="Servidor" value={form.incomingHost} error={errors.incomingHost} placeholder="mail.exemplo.com" autoComplete="off" onChange={(e) => update("incomingHost", e.target.value)} />
          <TextField label="Porta" value={form.incomingPort} error={errors.incomingPort} inputMode="numeric" onChange={(e) => update("incomingPort", e.target.value)} />
          <div className="field">
            <span className="field__label">Segurança da conexão</span>
            <Select label="Segurança da conexão de entrada" value={form.incomingSecurity} options={SECURITY_OPTIONS} onChange={(value) => update("incomingSecurity", value as ManualFormState["incomingSecurity"])} />
            <span className="field__help">Só conexões seguras; não oferecemos “sem segurança”.</span>
          </div>
          <div className="field">
            <span className="field__label">Autenticação</span>
            <Select label="Autenticação de entrada" value={form.incomingAuth} options={AUTH_OPTIONS} onChange={(value) => update("incomingAuth", value as ManualFormState["incomingAuth"])} />
            <span className="field__help">Senha criptografada, Kerberos/GSSAPI e NTLM entram nas próximas fatias.</span>
          </div>
          <TextField label="Usuário" value={form.incomingUser} error={errors.incomingUser} autoComplete="username" placeholder="ana@exemplo.com" onChange={(e) => update("incomingUser", e.target.value)} />
          <PasswordField label="Senha" value={form.incomingPassword} error={errors.incomingPassword} autoComplete="current-password" help="Guardada no cofre do sistema, nunca no banco do app." onChange={(e) => update("incomingPassword", e.target.value)} />
        </div>

        <div className="settings__group">
          <h3 className="settings__group-title">Saída</h3>
          <label className="choice"><input type="checkbox" className="checkbox" checked={form.sameCredentials} onChange={(e) => update("sameCredentials", e.target.checked)} />Usar as mesmas credenciais da entrada</label>
          <TextField label="Servidor" value={form.outgoingHost} error={errors.outgoingHost} placeholder="mail.exemplo.com" autoComplete="off" onChange={(e) => update("outgoingHost", e.target.value)} />
          <TextField label="Porta" value={form.outgoingPort} error={errors.outgoingPort} inputMode="numeric" onChange={(e) => update("outgoingPort", e.target.value)} />
          <div className="field">
            <span className="field__label">Segurança da conexão</span>
            <Select label="Segurança da conexão de saída" value={form.outgoingSecurity} options={SECURITY_OPTIONS} onChange={(value) => update("outgoingSecurity", value as ManualFormState["outgoingSecurity"])} />
          </div>
          <div className="field">
            <span className="field__label">Autenticação</span>
            <Select label="Autenticação de saída" value={form.outgoingAuth} options={AUTH_OPTIONS} onChange={(value) => update("outgoingAuth", value as ManualFormState["outgoingAuth"])} />
            <span className="field__help">Senha normal funciona agora; os outros métodos aparecem como “em breve”.</span>
          </div>
          <TextField
            label="Usuário"
            value={form.sameCredentials ? form.incomingUser : form.outgoingUser}
            error={form.sameCredentials ? undefined : errors.outgoingUser}
            disabled={form.sameCredentials}
            autoComplete="username"
            placeholder="ana@exemplo.com"
            onChange={(e) => update("outgoingUser", e.target.value)}
          />
          <PasswordField
            label="Senha"
            value={form.sameCredentials ? form.incomingPassword : form.outgoingPassword}
            error={form.sameCredentials ? undefined : errors.outgoingPassword}
            disabled={form.sameCredentials}
            autoComplete="current-password"
            help={form.sameCredentials ? "Igual à entrada." : "Guardada no cofre do sistema."}
            onChange={(e) => update("outgoingPassword", e.target.value)}
          />
        </div>

      <SectionMessage appearance="warning" title="O envio ainda não aceita todos os métodos">
        Senha normal funciona agora. Senha criptografada, NTLM e Kerberos/GSSAPI entram depois.
      </SectionMessage>

      {test && (
        <SectionMessage
          appearance={test.incoming.ok && test.outgoing.ok ? "success" : "error"}
          title={test.incoming.ok && test.outgoing.ok ? "Conexão testada" : "Confira a conexão"}
        >
          <span>Entrada: {test.incoming.message}</span><br />
          <span>Saída: {test.outgoing.message}</span>
        </SectionMessage>
      )}

      <div className="row modal__foot--split">
        <Button appearance="subtle" onClick={onBack} disabled={testing || saving}>Cancelar</Button>
        <Button icon="refresh" loading={testing} onClick={() => void runTest()}>Testar conexão</Button>
        <Button appearance="primary" type="submit" loading={saving} disabled={!testedOk}>Conectar</Button>
      </div>
    </form>
  );
}

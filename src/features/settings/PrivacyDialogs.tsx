// DS: modais.html "Senha, chave e dados guardados" · .modal (--warning) com .input--password (L54), .section-message--warning
// e .confirm-check (L57); o foco abre em "Cancelar" (ou no primeiro campo de senha) e o botão repete o verbo.
import { useState, type ReactNode } from "react";
import { mismatch, tooShort } from "../../lib/lock";
import { formatBytes } from "../../lib/format";
import { isAppError, type Account } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { ConfirmCheck } from "../../ui/ConfirmCheck";
import { Modal } from "../../ui/Modal";
import { PasswordField } from "../../ui/PasswordField";
import { SectionMessage } from "../../ui/SectionMessage";

const errorText = (error: unknown, fallback: string) =>
  isAppError(error) && (error.kind === "WrongPassword" || error.kind === "WeakPassword") ? error.message : fallback;

function Body({ children }: { children: ReactNode }) {
  return <div style={{ display: "grid", gap: "var(--space-200)" }}>{children}</div>;
}

/** Ligar a senha (CA2): duas vezes a senha nova e a confirmação de que não há como recuperá-la. */
export function SetPasswordDialog({ onConfirm, onClose }: { onConfirm: (password: string) => Promise<void>; onClose: () => void }) {
  const [password, setPassword] = useState("");
  const [repeat, setRepeat] = useState("");
  const [understood, setUnderstood] = useState(false);
  const [tried, setTried] = useState(false);
  const [failure, setFailure] = useState<string>();
  const [busy, setBusy] = useState(false);
  const short = tooShort(password);
  const differ = mismatch(password, repeat);

  const submit = async () => {
    setTried(true);
    if (short || differ || !understood) return;
    setBusy(true);
    try {
      await onConfirm(password);
      onClose();
    } catch (error) {
      setFailure(errorText(error, "Não foi possível ligar a senha. Tente de novo."));
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Pedir uma senha ao abrir?"
      icon="lock"
      appearance="warning"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
          <Button appearance="warning" loading={busy} disabled={!understood} onClick={() => void submit()}>Ligar a senha</Button>
        </>
      }
    >
      <Body>
        <PasswordField
          label="Senha"
          autoFocus
          autoComplete="new-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          help="Pelo menos 8 caracteres. Uma frase é mais fácil de lembrar e mais difícil de adivinhar."
          error={tried ? short : undefined}
        />
        <PasswordField
          label="Repita a senha"
          autoComplete="new-password"
          value={repeat}
          onChange={(e) => setRepeat(e.target.value)}
          error={tried ? differ : undefined}
        />
        <SectionMessage appearance="warning" title="Guarde bem a senha">
          Se você esquecer a senha, as mensagens guardadas neste computador não poderão ser abertas. Elas continuam no servidor e podem
          ser baixadas de novo.
        </SectionMessage>
        <ConfirmCheck checked={understood} onChange={setUnderstood}>Entendi que não há como recuperar a senha.</ConfirmCheck>
        {failure && <SectionMessage appearance="error" title={failure} />}
      </Body>
    </Modal>
  );
}

/** Trocar a senha: a atual e duas vezes a nova. A mesma chave do banco passa a ser protegida pela nova. */
export function ChangePasswordDialog({ onConfirm, onClose }: { onConfirm: (current: string, password: string) => Promise<void>; onClose: () => void }) {
  const [current, setCurrent] = useState("");
  const [password, setPassword] = useState("");
  const [repeat, setRepeat] = useState("");
  const [tried, setTried] = useState(false);
  const [failure, setFailure] = useState<string>();
  const [busy, setBusy] = useState(false);
  const short = tooShort(password);
  const differ = mismatch(password, repeat);

  const submit = async () => {
    setTried(true);
    if (short || differ || !current) return;
    setBusy(true);
    try {
      await onConfirm(current, password);
      onClose();
    } catch (error) {
      setFailure(errorText(error, "Não foi possível trocar a senha. Tente de novo."));
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Trocar a senha do EpistulaMail"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" loading={busy} onClick={() => void submit()}>Trocar senha</Button>
        </>
      }
    >
      <Body>
        <PasswordField label="Senha atual" autoFocus autoComplete="current-password" value={current} onChange={(e) => setCurrent(e.target.value)} error={failure} />
        <PasswordField
          label="Senha nova"
          autoComplete="new-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          help="Pelo menos 8 caracteres."
          error={tried ? short : undefined}
        />
        <PasswordField
          label="Repita a senha nova"
          autoComplete="new-password"
          value={repeat}
          onChange={(e) => setRepeat(e.target.value)}
          error={tried ? differ : undefined}
        />
      </Body>
    </Modal>
  );
}

/** Desligar a senha: confirma com a senha atual. */
export function RemovePasswordDialog({ onConfirm, onClose }: { onConfirm: (current: string) => Promise<void>; onClose: () => void }) {
  const [current, setCurrent] = useState("");
  const [failure, setFailure] = useState<string>();
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (!current) return;
    setBusy(true);
    try {
      await onConfirm(current);
      onClose();
    } catch (error) {
      setFailure(errorText(error, "Não foi possível desligar a senha. Tente de novo."));
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Desligar a senha do EpistulaMail?"
      icon="lock"
      appearance="warning"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
          <Button appearance="warning" loading={busy} disabled={!current} onClick={() => void submit()}>Desligar a senha</Button>
        </>
      }
    >
      <Body>
        <p style={{ margin: 0 }}>Quem entrar no seu Windows passa a abrir o EpistulaMail sem senha.</p>
        <PasswordField label="Senha atual" autoFocus autoComplete="current-password" value={current} onChange={(e) => setCurrent(e.target.value)} error={failure} />
      </Body>
    </Modal>
  );
}

/** Trocar a chave de proteção (CA4). */
export function RotateKeyDialog({ onConfirm, onClose }: { onConfirm: () => Promise<void>; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  return (
    <Modal
      title="Trocar a chave de proteção?"
      initialFocus="[data-cancel]"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" data-cancel onClick={onClose}>Cancelar</Button>
          <Button
            appearance="primary"
            loading={busy}
            onClick={async () => {
              setBusy(true);
              try {
                await onConfirm();
              } finally {
                onClose();
              }
            }}
          >
            Trocar chave
          </Button>
        </>
      }
    >
      <p>Tudo que está guardado neste computador é cifrado de novo com uma chave nova. Cópias antigas do arquivo do EpistulaMail deixam de abrir. Nada é apagado e nada muda no servidor.</p>
    </Modal>
  );
}

/** Apagar os dados de uma conta de que você saiu (CA7). */
export function DeleteAccountDataDialog({ account, bytes, onConfirm, onClose }: { account: Account; bytes: number; onConfirm: () => Promise<void>; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  return (
    <Modal
      title={`Apagar os dados de ${account.displayName} deste computador?`}
      icon="trash"
      initialFocus="[data-cancel]"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" data-cancel onClick={onClose}>Cancelar</Button>
          <Button
            appearance="danger"
            loading={busy}
            onClick={async () => {
              setBusy(true);
              try {
                await onConfirm();
              } finally {
                onClose();
              }
            }}
          >
            Apagar dados
          </Button>
        </>
      }
    >
      <p>
        As mensagens, pastas e rascunhos de <b>{account.email}</b> saem deste computador ({formatBytes(bytes)}), e a chave de proteção é
        trocada. Rascunhos que não foram enviados se perdem. As mensagens continuam no servidor.
      </p>
    </Modal>
  );
}

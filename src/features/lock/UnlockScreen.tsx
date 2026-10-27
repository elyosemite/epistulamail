// DS: carregamento.html "Tela de desbloqueio" (L53) · .unlock, __card, __title, __text, __form, com a .brand-logo (L76); campos.html
// .input--password (L54); mensagens.html .section-message--warning; modais.html "Apagar tudo e começar de novo?"
import { useEffect, useState } from "react";
import { retryLabel } from "../../lib/lock";
import { api, isAppError } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { Modal } from "../../ui/Modal";
import { PasswordField } from "../../ui/PasswordField";
import { SectionMessage } from "../../ui/SectionMessage";

/** Tela de desbloqueio (US10, CA2): cobre a janela inteira antes de qualquer conta ou mensagem. */
export function UnlockScreen({ retryIn: initialWait }: { retryIn: number }) {
  const [password, setPassword] = useState("");
  const [wait, setWait] = useState(initialWait);
  const [error, setError] = useState<string | undefined>(initialWait > 0 ? retryLabel(initialWait) : undefined);
  const [busy, setBusy] = useState(false);
  const [forgot, setForgot] = useState(false);
  const [confirmWipe, setConfirmWipe] = useState(false);

  // Contagem da espera entre tentativas, com o texto de ajuda acompanhando.
  useEffect(() => {
    if (wait <= 0) return;
    const timer = window.setTimeout(() => {
      setWait(wait - 1);
      setError(wait - 1 > 0 ? retryLabel(wait - 1) : "Senha incorreta.");
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [wait]);

  const submit = async () => {
    if (!password || wait > 0) return;
    setBusy(true);
    try {
      await api.unlockApp(password);
      // O evento `lock-changed` troca esta tela pelo app.
    } catch (err) {
      setPassword("");
      if (isAppError(err) && (err.kind === "WrongPassword" || err.kind === "UnlockWait")) {
        const status = await api.lockStatus().catch(() => null);
        const seconds = status?.retryIn ?? 0;
        setWait(seconds);
        setError(retryLabel(seconds));
      } else {
        setError(isAppError(err) ? err.message : "Não foi possível abrir os dados. Tente de novo.");
      }
    } finally {
      setBusy(false);
    }
  };

  const wipe = async () => {
    await api.wipeLocalData();
  };

  return (
    <div className="unlock">
      <div className="unlock__card">
        <img className="brand-logo" src="/epistulamail-logo.png" alt="" />
        {forgot ? (
          <>
            <h1 className="unlock__title">Esqueceu a senha?</h1>
            <SectionMessage appearance="warning" title="Não há como recuperar a senha">
              Ela não fica gravada em lugar nenhum. Sem ela, as mensagens guardadas neste computador não abrem. Elas continuam no
              servidor: você pode apagar os dados deste computador e entrar de novo nas contas.
            </SectionMessage>
            <div className="unlock__form">
              <Button appearance="danger" icon="trash" block onClick={() => setConfirmWipe(true)}>Apagar os dados e começar de novo</Button>
              <Button appearance="subtle" block onClick={() => setForgot(false)}>Voltar e tentar a senha</Button>
            </div>
          </>
        ) : (
          <>
            <h1 className="unlock__title">O EpistulaMail está bloqueado</h1>
            <p className="unlock__text">Digite a senha do EpistulaMail para ver as suas mensagens.</p>
            <form
              className="unlock__form"
              onSubmit={(e) => {
                e.preventDefault();
                void submit();
              }}
            >
              <PasswordField
                label="Senha do EpistulaMail"
                autoFocus
                autoComplete="current-password"
                value={password}
                error={error}
                onChange={(e) => setPassword(e.target.value)}
              />
              <Button appearance="primary" type="submit" block loading={busy} disabled={!password || wait > 0}>Desbloquear</Button>
              <Button appearance="link" onClick={() => setForgot(true)} style={{ justifySelf: "center" }}>Esqueci a senha</Button>
            </form>
          </>
        )}
      </div>
      {confirmWipe && (
        <WipeDialog onConfirm={wipe} onClose={() => setConfirmWipe(false)} />
      )}
    </div>
  );
}

function WipeDialog({ onConfirm, onClose }: { onConfirm: () => Promise<void>; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  return (
    <Modal
      title="Apagar tudo e começar de novo?"
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
              } catch {
                setFailed(true);
                setBusy(false);
              }
            }}
          >
            Apagar tudo
          </Button>
        </>
      }
    >
      <p>Todas as mensagens, pastas, rascunhos e ajustes deste computador são apagados, e a senha é desligada. Depois, entre de novo nas contas: as mensagens voltam do servidor.</p>
      {failed && <SectionMessage appearance="error" title="Não foi possível apagar os dados">Tente de novo em instantes.</SectionMessage>}
    </Modal>
  );
}

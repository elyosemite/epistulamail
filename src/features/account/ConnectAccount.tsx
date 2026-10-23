// DS: vazios.html (.empty e "Aguardando ação fora do app" com .empty__spinner), botoes.html (.btn), mensagens.html (.section-message)
// O botão usa i-mail: o DS não terá ícone de marca Google (L1 descartada).
import { useRef, useState } from "react";
import { api, isAppError, type Account, type AppError } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { EmptyState } from "../../ui/EmptyState";
import { SectionMessage } from "../../ui/SectionMessage";
import { Spinner } from "../../ui/Spinner";

type State = { step: "idle" } | { step: "waiting" } | { step: "failed"; error: AppError };

const UNKNOWN_ERROR: AppError = { kind: "AuthFailed", message: "Algo deu errado. Tente de novo." };

export function ConnectAccount({ onConnected }: { onConnected: (account: Account) => void }) {
  const [state, setState] = useState<State>({ step: "idle" });
  const cancelledByUser = useRef(false);

  async function connect() {
    cancelledByUser.current = false;
    setState({ step: "waiting" });
    try {
      onConnected(await api.connectAccount("google"));
    } catch (error) {
      if (cancelledByUser.current) return;
      setState({ step: "failed", error: isAppError(error) ? error : UNKNOWN_ERROR });
    }
  }

  async function cancel() {
    cancelledByUser.current = true;
    setState({ step: "idle" });
    await api.cancelConnect().catch(() => undefined);
  }

  if (state.step === "waiting") {
    return (
      <EmptyState
        media={<Spinner size="lg" className="empty__spinner" label="Aguardando autorização do Google" />}
        title="Continue no navegador"
        description="Abrimos a página de login do Google. Autorize o acesso do Epistula ao Gmail e volte para cá."
        actions={<Button appearance="subtle" onClick={cancel}>Cancelar</Button>}
      />
    );
  }

  return (
    <EmptyState
      title="Conecte seu Gmail"
      description="O login acontece no navegador, na página do Google. O Epistula nunca vê sua senha."
      actions={<Button appearance="primary" icon="mail" onClick={connect}>Conectar com Google</Button>}
    >
      {state.step === "failed" && <ConnectError error={state.error} onRetry={connect} />}
    </EmptyState>
  );
}

function ConnectError({ error, onRetry }: { error: AppError; onRetry: () => void }) {
  const interrupted = error.kind === "AuthCancelled" || error.kind === "AuthTimeout";
  return (
    <SectionMessage
      appearance={interrupted ? "warning" : "error"}
      title={interrupted ? "A conexão não foi concluída" : "Não foi possível conectar"}
      actions={[{ label: "Tentar de novo", onClick: onRetry }]}
    >
      {error.message}
    </SectionMessage>
  );
}

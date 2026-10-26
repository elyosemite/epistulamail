// Composição de DS: selecao.html (.provider-grid), arquivos.html (cartão "Abrir arquivo de e-mail" sob "Sem conta",
// US15; .nav-heading de sidebar.html), vazios.html (.empty, "Aguardando ação fora do app" com
// .empty__spinner), mensagens.html (.section-message), botoes.html (.btn)
import { useEffect, useRef, useState } from "react";
import { api, isAppError, type AppError, type Connected, type ProviderInfo } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { EmptyState } from "../../ui/EmptyState";
import { ProviderGrid } from "../../ui/ProviderGrid";
import { SectionMessage } from "../../ui/SectionMessage";
import { Spinner } from "../../ui/Spinner";

const UNKNOWN_ERROR: AppError = { kind: "AuthFailed", message: "Algo deu errado. Tente de novo." };

interface AddAccountProps {
  /** Primeira conta: sem "Cancelar", porque não há para onde voltar. */
  first: boolean;
  onConnected: (result: Connected) => void;
  onCancel: () => void;
  /** Abrir um arquivo de e-mail sem conta (US15, CA1). */
  onOpenFile: () => void;
}

export function AddAccount({ first, onConnected, onCancel, onOpenFile }: AddAccountProps) {
  const [providers, setProviders] = useState<ProviderInfo[] | null>(null);
  const [picked, setPicked] = useState<ProviderInfo | null>(null);

  useEffect(() => {
    api.listProviders().then(setProviders).catch(() => setProviders([]));
  }, []);

  if (picked) {
    return (
      <ConnectFlow
        providerName={picked.name}
        run={() => api.connectAccount(picked.id)}
        onConnected={onConnected}
        onBack={() => setPicked(null)}
      />
    );
  }

  return (
    <div style={{ display: "grid", gap: "var(--space-300)" }}>
      <EmptyState
        title={first ? "Conecte seu e-mail ou abra um arquivo" : "Adicionar conta"}
        description="Escolha o serviço do seu e-mail. O login acontece no navegador, na página do provedor; o Epistula nunca vê sua senha."
        actions={first ? undefined : <Button appearance="subtle" onClick={onCancel}>Cancelar</Button>}
      />
      {providers === null ? <Spinner size="lg" label="Carregando provedores" /> : <ProviderGrid providers={providers} onPick={setPicked} />}
      <div className="nav-heading" style={{ padding: 0 }}>Sem conta</div>
      <ProviderGrid
        providers={[{ id: "archive", name: "Abrir arquivo de e-mail", description: "Um backup ou uma pasta de outro programa. Sem conta e sem internet.", available: true, icon: "archive" }]}
        wide
        onPick={onOpenFile}
      />
    </div>
  );
}

type FlowState = { step: "waiting" } | { step: "failed"; error: AppError };

interface ConnectFlowProps {
  providerName: string;
  run: () => Promise<Connected>;
  onConnected: (result: Connected) => void;
  onBack: () => void;
}

/** Login no navegador: começa ao montar, mostra a espera e o erro com "Tentar de novo". */
export function ConnectFlow({ providerName, run, onConnected, onBack }: ConnectFlowProps) {
  const [state, setState] = useState<FlowState>({ step: "waiting" });
  const cancelledByUser = useRef(false);
  const started = useRef(false);
  const runRef = useRef(run);
  runRef.current = run;
  const connectedRef = useRef(onConnected);
  connectedRef.current = onConnected;

  const start = async () => {
    cancelledByUser.current = false;
    setState({ step: "waiting" });
    try {
      connectedRef.current(await runRef.current());
    } catch (error) {
      if (cancelledByUser.current) return;
      setState({ step: "failed", error: isAppError(error) ? error : UNKNOWN_ERROR });
    }
  };

  // Um login por montagem (o StrictMode monta duas vezes em desenvolvimento).
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void start();
  });

  const cancel = async () => {
    cancelledByUser.current = true;
    await api.cancelConnect().catch(() => undefined);
    onBack();
  };

  if (state.step === "waiting") {
    return (
      <EmptyState
        media={<Spinner size="lg" className="empty__spinner" label={`Aguardando autorização do ${providerName}`} />}
        title="Continue no navegador"
        description={`Abrimos a página de login do ${providerName}. Autorize o acesso do Epistula e volte para cá.`}
        actions={<Button appearance="subtle" onClick={cancel}>Cancelar</Button>}
      />
    );
  }

  const interrupted = state.error.kind === "AuthCancelled" || state.error.kind === "AuthTimeout";
  return (
    <EmptyState
      title={interrupted ? "A conexão não foi concluída" : "Não foi possível conectar"}
      actions={<Button appearance="subtle" onClick={onBack}>Voltar</Button>}
    >
      <SectionMessage
        appearance={interrupted ? "warning" : "error"}
        title={interrupted ? "O login foi interrompido" : "O provedor não autorizou o acesso"}
        actions={[{ label: "Tentar de novo", onClick: () => void start() }]}
      >
        {state.error.message}
      </SectionMessage>
    </EmptyState>
  );
}

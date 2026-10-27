// DS: modais.html · .modal (sem ícone: decisão neutra) com .modal__foot; mensagens.html · .section-message (erro).
// Confirmação da atualização automática, aberta pela notificação "Nova versão do EpistulaMail" do sino.
import { useEffect, useState } from "react";
import { notesBlocks } from "../../lib/update";
import { api, isAppError, type UpdateInfo } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { Modal } from "../../ui/Modal";
import { SectionMessage } from "../../ui/SectionMessage";
import { Spinner } from "../../ui/Spinner";

type Step = { kind: "loading" } | { kind: "none" } | { kind: "ready"; info: UpdateInfo } | { kind: "installing"; info: UpdateInfo } | { kind: "failed"; info: UpdateInfo | null; message: string };

export function UpdateDialog({ onClose }: { onClose: () => void }) {
  const [step, setStep] = useState<Step>({ kind: "loading" });

  useEffect(() => {
    api.updateStatus()
      .then((info) => setStep(info ? { kind: "ready", info } : { kind: "none" }))
      .catch((e) => setStep({ kind: "failed", info: null, message: isAppError(e) ? e.message : "Não foi possível verificar se há uma versão nova." }));
  }, []);

  const install = async (info: UpdateInfo) => {
    setStep({ kind: "installing", info });
    try {
      // No Windows, o app fecha durante a instalação e abre de novo na versão nova.
      await api.installUpdate();
    } catch (e) {
      setStep({ kind: "failed", info, message: isAppError(e) ? e.message : "Não foi possível atualizar o EpistulaMail." });
    }
  };

  const info = step.kind === "ready" || step.kind === "installing" ? step.info : step.kind === "failed" ? step.info : null;
  const busy = step.kind === "installing";

  return (
    <Modal
      title={info ? `Atualizar o EpistulaMail para a versão ${info.version}?` : "Atualização do EpistulaMail"}
      initialFocus="[data-cancel]"
      onClose={busy ? () => undefined : onClose}
      footer={
        info ? (
          <>
            <Button appearance="subtle" data-cancel disabled={busy} onClick={onClose}>Agora não</Button>
            <Button appearance="primary" loading={busy} onClick={() => void install(info)}>Atualizar e reiniciar</Button>
          </>
        ) : (
          <Button appearance="primary" data-cancel onClick={onClose}>Fechar</Button>
        )
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        {step.kind === "loading" && <Spinner label="Verificando se há uma versão nova" />}
        {step.kind === "none" && <p style={{ margin: 0 }}>Você já está na versão mais recente do EpistulaMail.</p>}
        {info && (
          <>
            <p style={{ margin: 0 }}>
              Você está na versão {info.current}. O EpistulaMail fecha, instala a versão {info.version} e abre de novo. As contas, as mensagens e as configurações continuam.
            </p>
            <UpdateNotes notes={info.notes} />
          </>
        )}
        {busy && <p style={{ margin: 0, color: "var(--color-text-subtle)" }}>Baixando e conferindo a assinatura da versão nova…</p>}
        {step.kind === "failed" && <SectionMessage appearance="error" title="A atualização não terminou">{step.message} Nada mudou na versão que você está usando.</SectionMessage>}
      </div>
    </Modal>
  );
}

/** O que mudou: títulos, listas e parágrafos das notas da release. */
function UpdateNotes({ notes }: { notes: string }) {
  const blocks = notesBlocks(notes);
  if (blocks.length === 0) return null;
  return (
    <div style={{ display: "grid", gap: "var(--space-100)", maxHeight: "16rem", overflowY: "auto" }}>
      {blocks.map((b, i) =>
        b.kind === "heading" ? (
          <p key={i} style={{ margin: 0, font: "var(--font-weight-semibold) var(--font-size-body)/var(--line-height-body) var(--font-family-body)" }}>{b.text}</p>
        ) : b.kind === "list" ? (
          <ul key={i} style={{ margin: 0, paddingInlineStart: "var(--space-300)", display: "grid", gap: "var(--space-050)" }}>
            {b.items.map((item, j) => <li key={j}>{item}</li>)}
          </ul>
        ) : (
          <p key={i} style={{ margin: 0 }}>{b.text}</p>
        ),
      )}
    </div>
  );
}

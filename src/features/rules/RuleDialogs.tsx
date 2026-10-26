// DS: regras.html "Rodar agora" e "Criar a partir de uma mensagem" · .modal (sem ícone: são decisões neutras),
// .modal__foot--split, .chip-group com .chip (.is-selected), .select, .confirm-check (L57); modais.html para excluir
import { useEffect, useState } from "react";
import { emailsLabel, emptySpec, targetFolders } from "../../lib/rules";
import { api, type Folder, type Rule, type RuleAction, type RuleCondition, type RuleSpec, type RuleSuggestions } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { ConfirmCheck } from "../../ui/ConfirmCheck";
import { Modal } from "../../ui/Modal";
import { Select } from "../../ui/Select";

/** "Rodar agora" (CA4): escolhe a pasta, mostra quantos e-mails serão afetados e confirma. */
export function RunNowDialog({ accountId, rule, folders, onRun, onClose }: { accountId: string; rule: Rule; folders: Folder[]; onRun: (folderId: string | null) => Promise<void>; onClose: () => void }) {
  const options = [{ value: "", label: "Caixa de entrada" }, ...folders.filter((f) => f.role === "custom" || f.role === "all").map((f) => ({ value: f.id, label: f.name }))];
  const [folder, setFolder] = useState("");
  const [count, setCount] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setCount(null);
    api.previewRule(accountId, rule.id, rule.spec, folder || null, true).then((p) => setCount(p.count)).catch(() => setCount(0));
  }, [accountId, rule, folder]);

  return (
    <Modal
      title={`Rodar "${rule.name}" agora?`}
      initialFocus="[data-cancel]"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" data-cancel onClick={onClose}>Cancelar</Button>
          <Button
            appearance="primary"
            loading={busy}
            disabled={!count}
            onClick={async () => {
              setBusy(true);
              try {
                await onRun(folder || null);
              } finally {
                onClose();
              }
            }}
          >
            {count ? `Rodar em ${emailsLabel(count)}` : "Rodar"}
          </Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-150)" }}>
        <div className="field">
          <span className="field__label" id="run-folder">Na pasta</span>
          <Select labelledBy="run-folder" value={folder} options={options} onChange={setFolder} />
        </div>
        <p style={{ margin: 0 }}>
          {count === null ? "Contando os e-mails…" : count === 0 ? "Nenhum e-mail desta pasta atende à regra." : <><b>{emailsLabel(count)}</b> serão afetados. Dá para desfazer pelo histórico das regras.</>}
        </p>
      </div>
    </Modal>
  );
}

export function DeleteRuleDialog({ rule, onConfirm, onClose }: { rule: Rule; onConfirm: () => Promise<void>; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  return (
    <Modal
      title={`Excluir a regra "${rule.name}"?`}
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
            Excluir regra
          </Button>
        </>
      }
    >
      <p>A regra para de organizar os e-mails. O que ela já fez continua no histórico e pode ser desfeito.</p>
    </Modal>
  );
}

type Pick = "from" | "domain" | "list" | "subject";

/** Condições escolhidas nas sugestões da mensagem ("qualquer uma" quando há mais de uma). */
export function quickConditions(s: RuleSuggestions, picked: Pick[]): RuleCondition[] {
  return picked.map((p): RuleCondition => {
    switch (p) {
      case "from": return { field: "from", op: "is", value: s.fromAddress };
      case "domain": return { field: "from_domain", op: "is", value: s.domain };
      case "list": return s.listId ? { field: "list", op: "yes", value: "" } : { field: "from", op: "is", value: s.fromAddress };
      case "subject": return { field: "subject", op: "is", value: s.subject };
    }
  });
}

export type QuickIntent = "rule" | "always-move" | "delete-all";

const QUICK_ACTIONS = [
  { value: "move", label: "Mover para" },
  { value: "archive", label: "Arquivar" },
  { value: "delete", label: "Excluir (Lixeira)" },
  { value: "mark_read", label: "Marcar como lida" },
  { value: "notify_always", label: "Notificar sempre" },
];

/** "Criar regra" a partir de uma mensagem (CA5): sugestões em chips, uma ação e "Mais opções" para o editor. */
export function QuickRuleDialog({
  suggestions,
  folders,
  intent,
  onCreate,
  onMore,
  onClose,
}: {
  suggestions: RuleSuggestions;
  folders: Folder[];
  intent: QuickIntent;
  onCreate: (name: string, spec: RuleSpec, applyExisting: boolean) => Promise<void>;
  onMore: (name: string, spec: RuleSpec) => void;
  onClose: () => void;
}) {
  const sender = suggestions.fromName || suggestions.fromAddress;
  const chips: Array<{ value: Pick; label: string }> = [
    { value: "from", label: `De ${sender}` },
    ...(suggestions.domain ? [{ value: "domain" as const, label: `Do domínio ${suggestions.domain}` }] : []),
    ...(suggestions.listId ? [{ value: "list" as const, label: "Desta lista de e-mails" }] : []),
    ...(suggestions.subject ? [{ value: "subject" as const, label: `Com assunto "${suggestions.subject.length > 30 ? `${suggestions.subject.slice(0, 30)}…` : suggestions.subject}"` }] : []),
  ];
  const [picked, setPicked] = useState<Pick[]>(["from"]);
  const [kind, setKind] = useState(intent === "delete-all" ? "delete" : "move");
  const [folder, setFolder] = useState("");
  const [applyExisting, setApplyExisting] = useState(intent !== "rule");
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState(false);

  const action: RuleAction = kind === "move" ? { kind: "move", folder } : ({ kind } as RuleAction);
  const conditions = quickConditions(suggestions, picked);
  const spec: RuleSpec = { ...emptySpec(), matchAll: conditions.length < 2, conditions, actions: [action] };
  const name = picked.length === 1 && picked[0] === "from" ? `De ${sender}` : chips.find((c) => c.value === picked[0])?.label ?? "Nova regra";
  const ready = picked.length > 0 && (kind !== "move" || folder !== "");
  const toggle = (p: Pick) => setPicked((all) => (all.includes(p) ? all.filter((x) => x !== p) : [...all, p]));

  return (
    <Modal
      title={intent === "delete-all" ? `Excluir tudo de ${sender}?` : intent === "always-move" ? `Sempre mover de ${sender}` : "Criar regra"}
      splitFooter
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" onClick={() => onMore(name, spec)}>Mais opções</Button>
          <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
          <Button
            appearance={kind === "delete" ? "danger" : "primary"}
            loading={busy}
            disabled={!ready}
            onClick={async () => {
              setBusy(true);
              setFailure(false);
              try {
                await onCreate(name, spec, applyExisting);
                onClose();
              } catch {
                setFailure(true);
                setBusy(false);
              }
            }}
          >
            Criar regra
          </Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <div style={{ display: "grid", gap: "var(--space-100)" }}>
          <span className="field__label" id="quick-which">E-mails</span>
          <div className="chip-group" role="group" aria-labelledby="quick-which">
            {chips.map((c) => (
              <button key={c.value} type="button" className={["chip", picked.includes(c.value) && "is-selected"].filter(Boolean).join(" ")} aria-pressed={picked.includes(c.value)} onClick={() => toggle(c.value)}>
                {c.label}
              </button>
            ))}
          </div>
        </div>
        <div className="field">
          <span className="field__label" id="quick-do">Fazer</span>
          <div className="row" style={{ gap: "var(--space-100)", flexWrap: "nowrap" }}>
            <Select labelledBy="quick-do" value={kind} options={QUICK_ACTIONS} onChange={setKind} />
            {kind === "move" && (
              <Select label="Pasta de destino" value={folder} options={[{ value: "", label: "Escolha a pasta" }, ...targetFolders(folders).map((f) => ({ value: f.id, label: f.name }))]} onChange={setFolder} />
            )}
          </div>
        </div>
        <ConfirmCheck checked={applyExisting} onChange={setApplyExisting}>Aplicar também aos que já estão nesta pasta</ConfirmCheck>
        {failure && <p className="field__help field__help--error" role="alert">Não foi possível criar a regra. Tente de novo.</p>}
      </div>
    </Modal>
  );
}

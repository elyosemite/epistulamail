// Composição de DS: regras.html (US12) · lista (L59) com a frase (L58), modelos (L62) no estado vazio e em "Nova regra",
// editor com prévia (L60, L61), histórico (L63) e os modais de rodar agora, duplicar e excluir; .select e .btn das
// Configurações (modais.html)
import { useCallback, useEffect, useState } from "react";
import { TEMPLATES, emailsLabel, emptySpec } from "../../lib/rules";
import { api, onBackendEvent, type Account, type Folder, type Rule, type RuleRun, type RuleSpec } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { Icon } from "../../ui/Icon";
import { Modal } from "../../ui/Modal";
import { Select } from "../../ui/Select";
import { SettingNote } from "../../ui/SettingsSheet";
import { Spinner } from "../../ui/Spinner";
import { useToast } from "../../ui/Toast";
import { DeleteRuleDialog, RunNowDialog } from "./RuleDialogs";
import { RuleEditor, type RuleDraft } from "./RuleEditor";
import { RuleHistory } from "./RuleHistory";
import { RuleList } from "./RuleList";

type View = { kind: "list" } | { kind: "new" } | { kind: "edit"; draft: RuleDraft } | { kind: "history" };
type Dialog = { kind: "run"; rule: Rule } | { kind: "delete"; rule: Rule } | { kind: "duplicate"; rule: Rule } | null;

/** Pedido vindo de fora (rascunho de "Criar regra… → Mais opções" ou "Ver" do toast). */
export type RulesRequest = { accountId: string; draft?: { name: string; spec: RuleSpec }; history?: boolean };

interface RulesSettingsProps {
  accounts: Account[];
  initialAccountId: string;
  request?: RulesRequest | null;
}

const newDraft = (name = "", spec: RuleSpec = emptySpec()): RuleDraft => ({ id: null, name, enabled: true, spec });

/** Sem pastas da outra conta: ao duplicar, as pastas são escolhidas de novo. */
function withoutFolders(spec: RuleSpec): RuleSpec {
  return {
    ...spec,
    folders: [],
    actions: spec.actions.map((a) => (a.kind === "move" || a.kind === "copy" ? { ...a, folder: "" } : a)),
  };
}

function Templates({ onPick, onBlank }: { onPick: (name: string, spec: RuleSpec) => void; onBlank: () => void }) {
  return (
    <div style={{ display: "grid", gap: "var(--space-300)" }}>
      <div className="empty" style={{ paddingBlock: "var(--space-200) 0" }}>
        <svg className="empty__art" viewBox="0 0 96 96" fill="none" aria-hidden="true">
          <path d="M14 20h68L56 52v20l-16 8V52z" stroke="currentColor" strokeWidth="2.5" strokeLinejoin="round" />
          <path d="m60 74 6 6 14-14" stroke="var(--color-icon-success)" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        <h2 className="empty__title">Deixe o EpistulaMail arrumar a Caixa de entrada</h2>
        <p className="empty__desc">Uma regra olha cada e-mail que chega e faz o que você pediu: mover, arquivar, marcar ou excluir. Comece por um modelo ou crie a sua.</p>
      </div>
      <div className="rule-templates">
        {TEMPLATES.map((t) => (
          <button key={t.id} type="button" className="rule-template" onClick={() => onPick(t.name, t.spec())}>
            <span className="rule-template__icon"><Icon name={t.icon} /></span>
            <span className="rule-template__title">{t.title}</span>
            <span className="rule-template__desc">{t.description}</span>
          </button>
        ))}
      </div>
      <div style={{ justifySelf: "center" }}><Button appearance="primary" icon="plus" onClick={onBlank}>Criar uma regra do zero</Button></div>
    </div>
  );
}

/** Configurações → Regras (US12, CA1). */
export function RulesSettings({ accounts, initialAccountId, request }: RulesSettingsProps) {
  const [accountId, setAccountId] = useState(request?.accountId ?? initialAccountId);
  const [rules, setRules] = useState<Rule[] | null>(null);
  const [folders, setFolders] = useState<Folder[]>([]);
  const [view, setView] = useState<View>(
    request?.draft ? { kind: "edit", draft: newDraft(request.draft.name, request.draft.spec) } : request?.history ? { kind: "history" } : { kind: "list" },
  );
  const [runs, setRuns] = useState<RuleRun[]>([]);
  const [undoing, setUndoing] = useState<number | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const toast = useToast();

  const load = useCallback(() => {
    api.listRules(accountId).then(setRules).catch(() => setRules([]));
    api.listFolders(accountId).then(setFolders).catch(() => setFolders([]));
  }, [accountId]);
  const loadHistory = useCallback(() => {
    api.ruleHistory(accountId).then(setRuns).catch(() => setRuns([]));
  }, [accountId]);

  useEffect(() => {
    setRules(null);
    load();
    const off = onBackendEvent("rules-applied", ({ accountId: id }) => { if (id === accountId) load(); });
    return off;
  }, [accountId, load]);
  useEffect(() => {
    if (view.kind === "history") loadHistory();
  }, [view.kind, loadHistory]);

  const failed = (title: string) => toast({ kind: "error", title, description: "Tente de novo em instantes." });

  const save = async (draft: RuleDraft, applyExisting: boolean) => {
    const saved = await api.saveRule(accountId, draft.id, draft.name, draft.enabled, draft.spec);
    setView({ kind: "list" });
    load();
    if (applyExisting) {
      try {
        const n = await api.runRuleNow(accountId, saved.id, null);
        toast({ kind: "success", title: `Regra "${saved.name}" salva`, description: n ? `Aplicada a ${emailsLabel(n)} que já estavam na pasta.` : "Nenhum e-mail que já estava na pasta atende à regra." });
      } catch {
        toast({ kind: "warning", title: `Regra "${saved.name}" salva`, description: "Não foi possível aplicar aos e-mails que já estavam na pasta." });
      }
    } else {
      toast({ kind: "success", title: `Regra "${saved.name}" salva` });
    }
  };

  const reorder = (ids: number[]) => {
    setRules((current) => current && ids.map((id) => current.find((r) => r.id === id)!).filter(Boolean));
    api.reorderRules(accountId, ids).then(setRules).catch(() => { failed("Não foi possível mudar a ordem"); load(); });
  };

  const toggle = (rule: Rule, enabled: boolean) => {
    setRules((current) => current && current.map((r) => (r.id === rule.id ? { ...r, enabled } : r)));
    api.setRuleEnabled(accountId, rule.id, enabled).then(() => load()).catch(() => { failed("Não foi possível salvar"); load(); });
  };

  const undo = async (run: RuleRun) => {
    setUndoing(run.id);
    try {
      const n = await api.undoRuleRun(accountId, run.id);
      toast({ kind: "success", title: n ? `${emailsLabel(n)} voltaram para onde estavam` : "Nada para desfazer", description: n ? undefined : "Os e-mails desse lote não existem mais." });
    } catch {
      failed("Não foi possível desfazer");
    }
    setUndoing(null);
    loadHistory();
    load();
  };

  if (view.kind === "edit") {
    return <RuleEditor key={view.draft.id ?? "new"} accountId={accountId} draft={view.draft} folders={folders} onSave={save} onCancel={() => setView({ kind: "list" })} />;
  }
  if (view.kind === "history") {
    return <RuleHistory runs={runs} undoing={undoing} onUndo={(run) => void undo(run)} onBack={() => setView({ kind: "list" })} />;
  }

  const accountOptions = accounts.map((a) => ({ value: a.id, label: a.displayName }));
  const pickTemplate = (name: string, spec: RuleSpec) => setView({ kind: "edit", draft: newDraft(name, spec) });
  return (
    <>
      <div className="row" style={{ justifyContent: "space-between", gap: "var(--space-150)" }}>
        <div className="row" style={{ gap: "var(--space-100)" }}>
          <span id="rules-account" style={{ font: "var(--font-body)", color: "var(--color-text-subtle)" }}>Regras de</span>
          <Select labelledBy="rules-account" value={accountId} options={accountOptions} onChange={(id) => { setAccountId(id); setView({ kind: "list" }); }} />
        </div>
        <div className="row" style={{ gap: "var(--space-100)" }}>
          <Button appearance="subtle" icon="history" onClick={() => setView({ kind: "history" })}>Histórico</Button>
          {view.kind === "list" && rules && rules.length > 0 && <Button appearance="primary" icon="plus" onClick={() => setView({ kind: "new" })}>Nova regra</Button>}
        </div>
      </div>

      {rules === null ? (
        <Spinner label="Carregando as regras" />
      ) : rules.length === 0 || view.kind === "new" ? (
        <>
          {view.kind === "new" && <Button appearance="subtle" icon="chevron-left" className="rule-editor__back" onClick={() => setView({ kind: "list" })}>Voltar às regras</Button>}
          <Templates onPick={pickTemplate} onBlank={() => setView({ kind: "edit", draft: newDraft() })} />
        </>
      ) : (
        <RuleList
          rules={rules}
          folders={folders}
          onToggle={toggle}
          onEdit={(rule) => setView({ kind: "edit", draft: { id: rule.id, name: rule.name, enabled: rule.enabled, spec: rule.spec } })}
          onRun={(rule) => setDialog({ kind: "run", rule })}
          onDuplicate={(rule) => (accounts.length > 1 ? setDialog({ kind: "duplicate", rule }) : setView({ kind: "edit", draft: newDraft(`${rule.name} (cópia)`, rule.spec) }))}
          onDelete={(rule) => setDialog({ kind: "delete", rule })}
          onReorder={reorder}
        />
      )}
      <SettingNote>As regras organizam os e-mails que chegam com o EpistulaMail aberto, inclusive na bandeja. O que chegou com ele fechado passa pelas regras na próxima sincronização.</SettingNote>

      {dialog?.kind === "run" && (
        <RunNowDialog
          accountId={accountId}
          rule={dialog.rule}
          folders={folders}
          onRun={async (folderId) => {
            try {
              const n = await api.runRuleNow(accountId, dialog.rule.id, folderId);
              toast({ kind: "success", title: `"${dialog.rule.name}" organizou ${emailsLabel(n)}`, actions: [{ label: "Ver histórico", onClick: () => setView({ kind: "history" }) }] });
              load();
            } catch {
              failed("Não foi possível rodar a regra");
            }
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "delete" && (
        <DeleteRuleDialog
          rule={dialog.rule}
          onConfirm={async () => {
            try {
              await api.deleteRule(accountId, dialog.rule.id);
              load();
            } catch {
              failed("Não foi possível excluir a regra");
            }
          }}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "duplicate" && (
        <DuplicateDialog
          accounts={accountOptions}
          current={accountId}
          onPick={(target) => {
            const rule = dialog.rule;
            setDialog(null);
            const spec = target === accountId ? rule.spec : withoutFolders(rule.spec);
            if (target !== accountId) setAccountId(target);
            setView({ kind: "edit", draft: newDraft(`${rule.name} (cópia)`, spec) });
          }}
          onClose={() => setDialog(null)}
        />
      )}
    </>
  );
}

/** "Duplicar" com várias contas (D1): para qual conta. As pastas são escolhidas de novo em outra conta. */
function DuplicateDialog({ accounts, current, onPick, onClose }: { accounts: Array<{ value: string; label: string }>; current: string; onPick: (accountId: string) => void; onClose: () => void }) {
  const [target, setTarget] = useState(current);
  return (
    <Modal
      title="Duplicar a regra"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" onClick={() => onPick(target)}>Duplicar</Button>
        </>
      }
    >
      <div className="field">
        <span className="field__label" id="dup-account">Para a conta</span>
        <Select labelledBy="dup-account" value={target} options={accounts} onChange={setTarget} />
      </div>
      {target !== current && <p className="field__help">Em outra conta, escolha de novo as pastas de destino.</p>}
    </Modal>
  );
}

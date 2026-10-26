// DS: regras.html "Editor de regra" · .rule-editor (__main, __back) com .rule-block (__head, __add) e .rule-cond (L60),
// .rule-preview (__count, __hint, __list, __item) (L61), .segmented, .select, .input, .confirm-check (L57),
// .section-message ("Como vai ficar" e regra encoberta) e a frase em RuleSentence (L58)
import { useEffect, useId, useState } from "react";
import {
  FIELDS,
  FOLLOWUP_DAYS,
  actionFor,
  availableActions,
  conditionFor,
  emailsLabel,
  fieldInfo,
  isCleanup,
  opsFor,
  saveProblem,
  sentence,
  sourceFolders,
  suggestedName,
  targetFolders,
  type ActionKind,
} from "../../lib/rules";
import { api, type Folder, type RuleAction, type RuleCondition, type RuleField, type RuleOp, type RulePreview, type RuleSpec } from "../../lib/tauri";
import { Button, IconButton } from "../../ui/Button";
import { ConfirmCheck } from "../../ui/ConfirmCheck";
import { Modal } from "../../ui/Modal";
import { RuleSentence } from "../../ui/RuleSentence";
import { SectionMessage } from "../../ui/SectionMessage";
import { Segmented } from "../../ui/Segmented";
import { Select } from "../../ui/Select";

export interface RuleDraft {
  id: number | null;
  name: string;
  enabled: boolean;
  spec: RuleSpec;
}

interface RuleEditorProps {
  accountId: string;
  draft: RuleDraft;
  folders: Folder[];
  onSave: (draft: RuleDraft, applyExisting: boolean) => Promise<void>;
  onCancel: () => void;
}

const MATCH = [
  { value: "all" as const, label: "todas" },
  { value: "any" as const, label: "qualquer uma" },
];
const PREVIEW_DELAY_MS = 300;
const PREVIEW_SHOWN = 5;

function ConditionRow({ condition, onChange, onRemove }: { condition: RuleCondition; onChange: (c: RuleCondition) => void; onRemove: () => void }) {
  const info = fieldInfo(condition.field);
  return (
    <div className="rule-cond">
      <Select label="Campo" value={condition.field} options={FIELDS} onChange={(field) => onChange(conditionFor(field as RuleField))} />
      <Select label="Operador" value={condition.op} options={opsFor(condition.field)} onChange={(op) => onChange({ ...condition, op: op as RuleOp })} />
      {info.kind === "bool" ? (
        <span />
      ) : (
        <div className="input">
          <input
            aria-label={info.kind === "number" ? `Valor em ${info.unit}` : "Valor"}
            inputMode={info.kind === "number" ? "numeric" : undefined}
            placeholder={info.kind === "number" ? info.unit : "Texto; vários separados por vírgula"}
            value={condition.value}
            onChange={(e) => onChange({ ...condition, value: info.kind === "number" ? e.target.value.replace(/\D/g, "") : e.target.value })}
          />
        </div>
      )}
      <IconButton icon="x" label="Remover esta condição" compact onClick={onRemove} />
    </div>
  );
}

function Conditions({ list, onChange, addLabel }: { list: RuleCondition[]; onChange: (list: RuleCondition[]) => void; addLabel: string }) {
  return (
    <>
      {list.map((c, i) => (
        <ConditionRow
          key={i}
          condition={c}
          onChange={(next) => onChange(list.map((x, j) => (j === i ? next : x)))}
          onRemove={() => onChange(list.filter((_, j) => j !== i))}
        />
      ))}
      <Button appearance="subtle" compact icon="plus" className="rule-block__add" onClick={() => onChange([...list, conditionFor("from")])}>{addLabel}</Button>
    </>
  );
}

function ActionRow({ action, index, actions, folders, onChange, onRemove }: { action: RuleAction; index: number; actions: RuleAction[]; folders: Folder[]; onChange: (a: RuleAction) => void; onRemove: () => void }) {
  const options = availableActions(actions, index).map((a) => ({ value: a.value, label: a.label }));
  const targets = targetFolders(folders).map((f) => ({ value: f.id, label: f.name }));
  return (
    <div className="rule-cond" style={{ gridTemplateColumns: "11rem minmax(0, 1fr) auto" }}>
      <Select label="Ação" value={action.kind} options={options} onChange={(kind) => onChange(actionFor(kind as ActionKind))} />
      {action.kind === "move" || action.kind === "copy" ? (
        <Select
          label={action.kind === "move" ? "Pasta de destino" : "Marcador"}
          value={action.folder}
          options={[{ value: "", label: "Escolha a pasta" }, ...targets]}
          onChange={(folder) => onChange({ ...action, folder })}
        />
      ) : action.kind === "followup" ? (
        <Select label="Prazo" value={String(action.days)} options={FOLLOWUP_DAYS} onChange={(d) => onChange({ kind: "followup", days: Number(d) })} />
      ) : action.kind === "delete" ? (
        <span className="field__help" style={{ margin: 0 }}>Vai para a Lixeira, e dá para desfazer pelo histórico.</span>
      ) : (
        <span />
      )}
      <IconButton icon="x" label="Remover esta ação" compact onClick={onRemove} />
    </div>
  );
}

/** Editor de regra (CA2): quando, fazer e exceto se, com a prévia ao lado. */
export function RuleEditor({ accountId, draft, folders, onSave, onCancel }: RuleEditorProps) {
  const [name, setName] = useState(draft.name);
  const [named, setNamed] = useState(draft.id !== null || draft.name !== "");
  const [spec, setSpec] = useState<RuleSpec>(draft.spec);
  const [applyExisting, setApplyExisting] = useState(false);
  const [preview, setPreview] = useState<RulePreview | null>(null);
  const [showAll, setShowAll] = useState(false);
  const [busy, setBusy] = useState(false);
  const [confirmAll, setConfirmAll] = useState(false);
  const [failure, setFailure] = useState<string>();
  const ids = { name: useId(), when: useId(), act: useId(), except: useId(), count: useId() };

  const shownName = named ? name : suggestedName(spec);
  const problem = saveProblem(shownName, spec);
  const cleanup = isCleanup(spec);
  const s = sentence(spec, folders);
  const extra = sourceFolders(folders);
  const scope = spec.folders[0] ?? "";

  // Prévia ao vivo, sem rede (CA2).
  useEffect(() => {
    const timer = window.setTimeout(() => {
      api.previewRule(accountId, draft.id, spec).then(setPreview).catch(() => setPreview(null));
    }, PREVIEW_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [accountId, draft.id, spec]);

  const change = (patch: Partial<RuleSpec>) => setSpec((current) => ({ ...current, ...patch }));

  const save = async (confirmed = false) => {
    if (problem) return;
    if (spec.conditions.length === 0 && !confirmed) return setConfirmAll(true);
    setBusy(true);
    setFailure(undefined);
    try {
      await onSave({ ...draft, name: shownName.trim(), spec }, applyExisting);
    } catch {
      setFailure("Não foi possível salvar a regra. Confira as pastas e tente de novo.");
      setBusy(false);
    }
  };

  return (
    <div className="rule-editor">
      <div className="rule-editor__main">
        <Button appearance="subtle" icon="chevron-left" className="rule-editor__back" onClick={onCancel}>Voltar às regras</Button>
        <div className="field" style={{ maxWidth: "24rem" }}>
          <label className="field__label" htmlFor={ids.name}>Nome da regra</label>
          <div className="input">
            <input id={ids.name} maxLength={80} value={shownName} onChange={(e) => { setName(e.target.value); setNamed(true); }} />
          </div>
        </div>

        <div className="rule-block" role="group" aria-labelledby={ids.when}>
          <div className="rule-block__head" id={ids.when}>
            {cleanup ? "Uma vez por dia, olhar os e-mails da" : "Quando chegar um e-mail na"}
            <Select
              label="Pasta"
              value={scope}
              options={[{ value: "", label: "Caixa de entrada" }, ...extra.map((f) => ({ value: f.id, label: `Caixa de entrada e ${f.name}` }))]}
              onChange={(v) => change({ folders: v ? [v] : [] })}
            />
            que atenda a
            <Segmented label="Quais condições" options={MATCH} value={spec.matchAll ? "all" : "any"} onChange={(v) => change({ matchAll: v === "all" })} />
          </div>
          <Conditions list={spec.conditions} onChange={(conditions) => change({ conditions })} addLabel="Adicionar condição" />
          <p className="field__help" style={{ margin: 0 }}>Vários valores separados por vírgula valem como "qualquer um deles". Maiúsculas e acentos não fazem diferença.</p>
        </div>

        <div className="rule-block" role="group" aria-labelledby={ids.act}>
          <div className="rule-block__head" id={ids.act}>Fazer</div>
          {spec.actions.map((a, i) => (
            <ActionRow
              key={i}
              action={a}
              index={i}
              actions={spec.actions}
              folders={folders}
              onChange={(next) => change({ actions: spec.actions.map((x, j) => (j === i ? next : x)) })}
              onRemove={() => change({ actions: spec.actions.filter((_, j) => j !== i) })}
            />
          ))}
          {availableActions(spec.actions, -1).length > 0 && (
            <Button
              appearance="subtle"
              compact
              icon="plus"
              className="rule-block__add"
              onClick={() => change({ actions: [...spec.actions, actionFor(availableActions(spec.actions, -1)[0].value)] })}
            >
              Adicionar ação
            </Button>
          )}
          <ConfirmCheck checked={spec.stop} onChange={(stop) => change({ stop })}>Parar de aplicar as outras regras a este e-mail</ConfirmCheck>
        </div>

        <div className="rule-block" role="group" aria-labelledby={ids.except}>
          <div className="rule-block__head" id={ids.except}>
            Exceto se <span style={{ font: "var(--font-body-small)", color: "var(--color-text-subtle)" }}>(opcional)</span>
          </div>
          <Conditions list={spec.exceptions} onChange={(exceptions) => change({ exceptions })} addLabel="Adicionar exceção" />
        </div>

        {cleanup ? (
          <SectionMessage title="Regra de limpeza">Roda uma vez por dia e em "Rodar agora", nos e-mails que já estão na pasta.</SectionMessage>
        ) : (
          <ConfirmCheck checked={applyExisting} onChange={setApplyExisting}>Aplicar também aos e-mails que já estão na pasta</ConfirmCheck>
        )}

        {preview?.shadowed && (
          <SectionMessage appearance="warning" title={`A regra "${preview.shadowed.ruleName}" vem antes e para as outras`}>
            Para {emailsLabel(preview.shadowed.count)} destes, esta regra não vai rodar. Mova esta regra para cima ou tire o "parar" da outra.
          </SectionMessage>
        )}

        <div className="section-message" role="note">
          <svg className="i" aria-hidden="true"><use href="#i-info" /></svg>
          <div>
            <p className="section-message__title">Como vai ficar</p>
            <p><RuleSentence conditions={s.conditions} exceptions={s.exceptions} actions={s.actions} joiner={s.joiner} stop={spec.stop} /></p>
          </div>
        </div>

        {failure && <SectionMessage appearance="error" title={failure} />}

        <div className="row" style={{ justifyContent: "flex-end", gap: "var(--space-100)" }}>
          {problem && <span className="field__help" style={{ marginRight: "auto" }}>{problem}</span>}
          <Button appearance="subtle" onClick={onCancel}>Cancelar</Button>
          <Button appearance="primary" loading={busy} disabled={!!problem} onClick={() => void save()}>
            {spec.conditions.length === 0 ? "Salvar para todos os e-mails" : "Salvar regra"}
          </Button>
        </div>
      </div>

      <aside className="rule-preview" aria-live="polite" aria-labelledby={ids.count}>
        {preview === null ? (
          <p className="rule-preview__count" id={ids.count}>Calculando…</p>
        ) : preview.count === 0 ? (
          <>
            <p className="rule-preview__count" id={ids.count}>Não pegaria nenhum</p>
            <p className="rule-preview__hint">Nenhum dos {preview.checked} e-mails mais recentes atende a estas condições. Confira o texto ou troque "todas" por "qualquer uma".</p>
          </>
        ) : (
          <>
            <p className="rule-preview__count" id={ids.count}>Pegaria {preview.count} de {preview.checked}</p>
            <p className="rule-preview__hint">Entre os {preview.checked} e-mails mais recentes da Caixa de entrada.</p>
            <ul className="rule-preview__list">
              {preview.sample.slice(0, showAll ? undefined : PREVIEW_SHOWN).map((m, i) => (
                <li key={i} className="rule-preview__item"><b>{m.from}</b><span>{m.subject || "(sem assunto)"}</span></li>
              ))}
            </ul>
            {!showAll && preview.sample.length > PREVIEW_SHOWN && (
              <Button appearance="link" style={{ justifySelf: "start" }} onClick={() => setShowAll(true)}>
                Ver {preview.count > preview.sample.length ? `os primeiros ${preview.sample.length}` : `os ${preview.count}`}
              </Button>
            )}
          </>
        )}
      </aside>

      {confirmAll && (
        <Modal
          title="Valer para todos os e-mails?"
          appearance="warning"
          icon="warning"
          initialFocus="[data-cancel]"
          onClose={() => setConfirmAll(false)}
          footer={
            <>
              <Button appearance="subtle" data-cancel onClick={() => setConfirmAll(false)}>Cancelar</Button>
              <Button appearance="warning" onClick={() => { setConfirmAll(false); void save(true); }}>Salvar para todos</Button>
            </>
          }
        >
          <p>A regra não tem condição: vai agir em todo e-mail que chegar na pasta.</p>
        </Modal>
      )}
    </div>
  );
}

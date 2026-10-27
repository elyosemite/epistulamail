// DS: busca.html "Ações nos resultados e aplicar regra" (US16) · .modal.modal--medium com .choice-list de
// .choice-card (L64), a frase da regra (L58) e .rule-preview (L61)
import { useEffect, useState } from "react";
import { emailsLabel, sentence } from "../../lib/rules";
import { api, type Folder, type Rule, type RulePreview } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { ChoiceList } from "../../ui/ChoiceCard";
import { Modal } from "../../ui/Modal";
import { RuleSentence } from "../../ui/RuleSentence";

const CREATE = "__new";

interface ApplyRuleDialogProps {
  accountId: string;
  messageIds: string[];
  /** Pastas da conta, para a frase das regras. */
  folders: Folder[];
  onApplied: (count: number, rule: Rule) => void;
  /** "Criar uma regra com esta busca…": abre o editor com as condições dos chips. */
  onCreate: () => void;
  onClose: () => void;
}

/** "Aplicar regra…" nos resultados marcados: escolhe uma regra da conta, vê a prévia e aplica. */
export function ApplyRuleDialog({ accountId, messageIds, folders, onApplied, onCreate, onClose }: ApplyRuleDialogProps) {
  const [rules, setRules] = useState<Rule[] | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [preview, setPreview] = useState<RulePreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listRules(accountId)
      .then((all) => {
        setRules(all);
        setPicked(all[0] ? String(all[0].id) : CREATE);
      })
      .catch(() => {
        setRules([]);
        setPicked(CREATE);
      });
  }, [accountId]);

  const rule = rules?.find((r) => String(r.id) === picked) ?? null;
  useEffect(() => {
    setPreview(null);
    setError(null);
    if (!rule) return;
    let live = true;
    api.previewRuleOn(accountId, rule.id, messageIds)
      .then((p) => live && setPreview(p))
      .catch(() => live && setError("Não foi possível calcular a prévia."));
    return () => {
      live = false;
    };
  }, [accountId, rule, messageIds]);

  const apply = async () => {
    if (!rule) return onCreate();
    setBusy(true);
    try {
      const done = await api.applyRuleOn(accountId, rule.id, messageIds);
      onApplied(done, rule);
      onClose();
    } catch {
      setError("A regra não foi aplicada. Tente de novo em instantes.");
      setBusy(false);
    }
  };

  const options = [
    ...(rules ?? []).map((r) => {
      const s = sentence(r.spec, folders);
      return { value: String(r.id), icon: "filter", title: r.name, description: <RuleSentence conditions={s.conditions} exceptions={s.exceptions} actions={s.actions} joiner={s.joiner} stop={r.spec.stop} /> };
    }),
    { value: CREATE, icon: "plus", title: "Criar uma regra com esta busca…", description: "As condições começam com os chips da busca." },
  ];

  return (
    <Modal
      title={`Aplicar regra a ${messageIds.length === 1 ? "1 resultado" : `${messageIds.length.toLocaleString("pt-BR")} resultados`}`}
      medium
      initialFocus="[data-cancel]"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" data-cancel onClick={onClose}>Cancelar</Button>
          {rule ? (
            <Button appearance="primary" loading={busy} disabled={!preview || preview.count === 0} onClick={() => void apply()}>
              {preview && preview.count > 0 ? `Aplicar a ${preview.count.toLocaleString("pt-BR")}` : "Aplicar"}
            </Button>
          ) : (
            <Button appearance="primary" disabled={rules === null} onClick={onCreate}>Criar regra…</Button>
          )}
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        {rules === null ? <p style={{ margin: 0 }}>Carregando as regras…</p> : <ChoiceList label="Regra" name="apply-rule" options={options} value={picked} onChange={setPicked} />}
        {rule && (
          <aside className="rule-preview" aria-live="polite">
            {error ? (
              <p className="rule-preview__count" role="alert">{error}</p>
            ) : !preview ? (
              <p className="rule-preview__count">Calculando…</p>
            ) : preview.count === 0 ? (
              <>
                <p className="rule-preview__count">Não pegaria nenhum</p>
                <p className="rule-preview__hint">Nenhum dos resultados marcados cumpre as condições desta regra.</p>
              </>
            ) : (
              <>
                <p className="rule-preview__count">Pegaria {preview.count.toLocaleString("pt-BR")} dos {preview.checked.toLocaleString("pt-BR")}</p>
                <p className="rule-preview__hint">A regra só age nos resultados que cumprem as condições dela. {emailsLabel(preview.count)} vão para o histórico das regras, com "Desfazer".</p>
                <ul className="rule-preview__list">
                  {preview.sample.map((m, i) => (
                    <li key={i} className="rule-preview__item"><b>{m.from}</b><span>{m.subject || "(sem assunto)"}</span></li>
                  ))}
                </ul>
              </>
            )}
          </aside>
        )}
      </div>
    </Modal>
  );
}

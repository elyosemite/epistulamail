// DS: regras.html "Histórico das regras" · .settings__group com .rule-history e .rule-history__row (.is-undone) (L63)
import { runText, runWhen } from "../../lib/rules";
import type { RuleRun } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { EmptyState } from "../../ui/EmptyState";

/** O que as regras fizeram nos últimos 30 dias, com "Desfazer" por lote (CA6). */
export function RuleHistory({ runs, undoing, onUndo, onBack }: { runs: RuleRun[]; undoing: number | null; onUndo: (run: RuleRun) => void; onBack: () => void }) {
  return (
    <div style={{ display: "grid", gap: "var(--space-200)" }}>
      <Button appearance="subtle" icon="chevron-left" className="rule-editor__back" onClick={onBack}>Voltar às regras</Button>
      <div className="settings__group">
        <div className="row" style={{ justifyContent: "space-between" }}>
          <h4 className="settings__group-title" style={{ margin: 0 }}>Histórico das regras</h4>
          <span style={{ font: "var(--font-body-small)", color: "var(--color-text-subtle)" }}>Últimos 30 dias</span>
        </div>
        {runs.length === 0 ? (
          <EmptyState title="Nada por aqui ainda" description="Quando uma regra organizar um e-mail, o que ela fez aparece aqui, com Desfazer." />
        ) : (
          <div className="rule-history">
            {runs.map((run) => (
              <div key={run.id} className={["rule-history__row", run.undone && "is-undone"].filter(Boolean).join(" ")}>
                <span>{runWhen(run.at)}</span>
                <span><b>{run.ruleName}</b> {runText(run)}{run.undone ? " · desfeito" : ""}</span>
                {run.undone ? <span /> : <Button appearance="subtle" compact loading={undoing === run.id} onClick={() => onUndo(run)}>Desfazer</Button>}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

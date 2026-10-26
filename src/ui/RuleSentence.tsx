// DS: regras.html · .rule-sentence com .rule-term (--action, --danger) e o ícone arrow-right entre condições e ações (L58)
import { Fragment } from "react";
import type { Term } from "../lib/rules";
import { Icon } from "./Icon";

interface RuleSentenceProps {
  conditions: Term[];
  exceptions: Term[];
  actions: Term[];
  /** "e" (todas) ou "ou" (qualquer uma). */
  joiner: string;
  stop?: boolean;
}

function TermChip({ term }: { term: Term }) {
  const cls = ["rule-term", term.tone === "action" && "rule-term--action", term.tone === "danger" && "rule-term--danger"].filter(Boolean).join(" ");
  return (
    <span className={cls}>
      {term.icon && <Icon name={term.icon} />}
      {term.text}
    </span>
  );
}

function joined(terms: Term[], joiner: string) {
  return terms.map((t, i) => (
    <Fragment key={`${t.text}-${i}`}>
      {i > 0 && ` ${joiner} `}
      <TermChip term={t} />
    </Fragment>
  ));
}

/** Uma regra como frase: "Se De contém @loja.com → Mover para Compras". */
export function RuleSentence({ conditions, exceptions, actions, joiner, stop }: RuleSentenceProps) {
  return (
    <span className="rule-sentence">
      {conditions.length > 0 ? <>Se {joined(conditions, joiner)}</> : "Todos os e-mails"}
      {exceptions.length > 0 && <>, exceto se {joined(exceptions, "ou")}</>}
      <Icon name="arrow-right" size="sm" />
      {actions.map((t, i) => <TermChip key={`${t.text}-${i}`} term={t} />)}
      {stop && " e parar"}
    </span>
  );
}

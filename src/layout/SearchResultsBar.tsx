// DS: busca.html "Resultados e onde buscar" (L74) · .search-bar (__count, __end) com .segmented, .select e .btn
import { Button } from "../ui/Button";
import { Segmented } from "../ui/Segmented";
import { Select } from "../ui/Select";

interface SearchResultsBarProps {
  /** `null` enquanto busca. */
  count: number | null;
  /** Há mais que o limite (200): a contagem vira "200+". */
  truncated: boolean;
  allAccounts: boolean;
  oldestFirst: boolean;
  /** Já é uma busca salva: o botão some. */
  saved: boolean;
  onScope: (allAccounts: boolean) => void;
  onOrder: (oldestFirst: boolean) => void;
  onSave: () => void;
}

const SCOPES = [
  { value: "account", label: "Esta conta" },
  { value: "all", label: "Todas as contas" },
] as const;
const ORDERS = [
  { value: "newest", label: "Mais recentes" },
  { value: "oldest", label: "Mais antigas" },
];

/** Barra entre a barra de comandos e a lista: quantos, onde buscar, a ordem e "Salvar busca". */
export function SearchResultsBar({ count, truncated, allAccounts, oldestFirst, saved, onScope, onOrder, onSave }: SearchResultsBarProps) {
  const label = count === null ? "Buscando…" : truncated ? `Mais de ${count.toLocaleString("pt-BR")} resultados` : count === 1 ? "1 resultado" : `${count.toLocaleString("pt-BR")} resultados`;
  return (
    <div className="search-bar">
      <span className="search-bar__count" aria-live="polite">{label}</span>
      <Segmented label="Onde buscar" options={SCOPES} value={allAccounts ? "all" : "account"} onChange={(v) => onScope(v === "all")} />
      <div className="search-bar__end">
        <Select label="Ordem" value={oldestFirst ? "oldest" : "newest"} options={ORDERS} onChange={(v) => onOrder(v === "oldest")} />
        {!saved && <Button appearance="subtle" compact icon="plus" onClick={onSave}>Salvar busca</Button>}
      </div>
    </div>
  );
}

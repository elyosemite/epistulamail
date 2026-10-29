// DS: pastas.html "Sinalizadas" · .nav-item com o ícone flag e a quantidade; .is-overdue em "Atrasadas" (US6)
import type { FlagBucket } from "../lib/tauri";
import { NavItem } from "../ui/Sidebar";

interface FlaggedItemProps {
  bucket: FlagBucket;
  label: string;
  count: number;
  current: boolean;
  onSelect: () => void;
}

/** Um prazo das sinalizadas: Atrasadas, Hoje, Esta semana, Mais tarde ou Sem data. */
export function FlaggedItem({ bucket, label, count, current, onSelect }: FlaggedItemProps) {
  return <NavItem icon="flag" label={label} count={count} current={current} overdue={bucket === "overdue"} plainCount onSelect={onSelect} />;
}

// DS: campos.html "Lista de opções" · .select (select nativo + ícone chevron-down)
import { Icon } from "./Icon";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

interface SelectProps {
  value: string;
  options: readonly SelectOption[];
  label?: string;
  labelledBy?: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}

export function Select({ value, options, label, labelledBy, disabled, onChange }: SelectProps) {
  return (
    <span className="select">
      <select value={value} aria-label={label} aria-labelledby={labelledBy} disabled={disabled} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => (
          <option key={o.value} value={o.value} disabled={o.disabled}>{o.label}</option>
        ))}
      </select>
      <Icon name="chevron-down" />
    </span>
  );
}

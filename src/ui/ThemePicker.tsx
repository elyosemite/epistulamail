// DS: selecao.html "Tema" · .theme-picker (fieldset), .theme-option (rádio em cartão), __icon, __name, __desc
import { useId } from "react";
import { Icon } from "./Icon";

interface ThemePickerProps<T extends string> {
  legend: string;
  options: ReadonlyArray<{ value: T; label: string; description: string; icon: string }>;
  value: T;
  onChange: (value: T) => void;
}

export function ThemePicker<T extends string>({ legend, options, value, onChange }: ThemePickerProps<T>) {
  const name = useId();
  return (
    <fieldset className="theme-picker">
      <legend className="visually-hidden">{legend}</legend>
      {options.map((o) => (
        <label key={o.value} className="theme-option">
          <input type="radio" name={name} value={o.value} checked={o.value === value} onChange={() => onChange(o.value)} />
          <span className="theme-option__icon" aria-hidden="true"><Icon name={o.icon} size="lg" /></span>
          <span className="theme-option__name">{o.label}</span>
          <span className="theme-option__desc">{o.description}</span>
        </label>
      ))}
    </fieldset>
  );
}

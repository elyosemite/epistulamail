// DS: criptografia.html · .choice-list com .choice-card (__icon, __title, __desc) (L64): uma opção de um assistente, como rádio grande
import type { ReactNode } from "react";
import { Icon } from "./Icon";

export interface Choice<T extends string> {
  value: T;
  title: ReactNode;
  description?: ReactNode;
  icon?: string;
}

interface ChoiceListProps<T extends string> {
  label: string;
  name: string;
  options: ReadonlyArray<Choice<T>>;
  value: T | null;
  onChange: (value: T) => void;
}

export function ChoiceList<T extends string>({ label, name, options, value, onChange }: ChoiceListProps<T>) {
  return (
    <div className="choice-list" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <label key={o.value} className="choice-card">
          <input type="radio" name={name} checked={value === o.value} onChange={() => onChange(o.value)} />
          {o.icon ? <span className="choice-card__icon"><Icon name={o.icon} /></span> : <span />}
          <span>
            <span className="choice-card__title">{o.title}</span>
            {o.description && <span className="choice-card__desc">{o.description}</span>}
          </span>
        </label>
      ))}
    </div>
  );
}

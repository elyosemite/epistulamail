// DS: campos.html · .field, .field__label, .input, .field__help (--error)
// DS: selecao.html · .chip-group com .chip (.is-selected) para sugestões abaixo do campo
import { useId, type InputHTMLAttributes, type ReactNode } from "react";
import { Icon } from "./Icon";

interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "id"> {
  label: string;
  error?: string;
  children?: ReactNode;
}

export function TextField({ label, error, children, ...input }: TextFieldProps) {
  const id = useId();
  const helpId = `${id}-help`;
  return (
    <div className="field">
      <label className="field__label" htmlFor={id}>{label}</label>
      <div className={["input", error && "is-invalid"].filter(Boolean).join(" ")}>
        <input id={id} aria-invalid={error ? true : undefined} aria-describedby={error ? helpId : undefined} {...input} />
      </div>
      {error && (
        <span className="field__help field__help--error" id={helpId}>
          <Icon name="error" size="sm" />
          {error}
        </span>
      )}
      {children}
    </div>
  );
}

export function ChipGroup({ label, options, value, onPick }: { label: string; options: string[]; value: string; onPick: (option: string) => void }) {
  return (
    <div className="chip-group" role="group" aria-label={label}>
      {options.map((o) => (
        <button key={o} type="button" className={["chip", o === value && "is-selected"].filter(Boolean).join(" ")} aria-pressed={o === value} onClick={() => onPick(o)}>
          {o}
        </button>
      ))}
    </div>
  );
}

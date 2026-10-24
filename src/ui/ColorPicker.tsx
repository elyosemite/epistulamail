// DS: selecao.html "Cor da conta" · .color-picker, .color-swatch (rádio nativo + .avatar), .color-swatch__check
import { useId } from "react";
import { Icon } from "./Icon";

interface ColorPickerProps<T extends string> {
  legend: string;
  options: { value: T; label: string }[];
  value: T;
  /** Texto mostrado nas amostras (a inicial da conta). */
  sample: string;
  onChange: (value: T) => void;
}

export function ColorPicker<T extends string>({ legend, options, value, sample, onChange }: ColorPickerProps<T>) {
  const name = useId();
  return (
    <fieldset className="color-picker">
      <legend>{legend}</legend>
      {options.map((o) => (
        <label key={o.value} className="color-swatch">
          <input type="radio" name={name} value={o.value} checked={o.value === value} onChange={() => onChange(o.value)} />
          <span className={["avatar avatar--lg", o.value !== "blue" && `avatar--${o.value}`].filter(Boolean).join(" ")} aria-hidden="true">{sample}</span>
          <span className="color-swatch__check" aria-hidden="true"><Icon name="check" /></span>
          <span className="visually-hidden">{o.label}</span>
        </label>
      ))}
    </fieldset>
  );
}

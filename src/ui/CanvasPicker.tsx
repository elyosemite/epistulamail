// DS: selecao.html "Cor do fundo" · .color-picker, .color-swatch com .color-swatch__fill (cor em --swatch), .color-swatch__check
import { useId, type CSSProperties } from "react";
import { Icon } from "./Icon";

interface CanvasPickerProps<T extends string> {
  legend: string;
  /** `swatch`: valor CSS da cor (um token, ex. `var(--color-canvas-blue)`). */
  options: ReadonlyArray<{ value: T; label: string; swatch: string }>;
  value: T;
  onChange: (value: T) => void;
}

export function CanvasPicker<T extends string>({ legend, options, value, onChange }: CanvasPickerProps<T>) {
  const name = useId();
  return (
    <fieldset className="color-picker">
      <legend className="visually-hidden">{legend}</legend>
      {options.map((o) => (
        <label key={o.value} className="color-swatch" data-tip={o.label}>
          <input type="radio" name={name} value={o.value} checked={o.value === value} onChange={() => onChange(o.value)} />
          <span className="color-swatch__fill" style={{ "--swatch": o.swatch } as CSSProperties} aria-hidden="true" />
          <span className="color-swatch__check" aria-hidden="true"><Icon name="check" /></span>
          <span className="visually-hidden">{o.label}</span>
        </label>
      ))}
    </fieldset>
  );
}

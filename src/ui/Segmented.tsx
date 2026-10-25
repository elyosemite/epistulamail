// DS: selecao.html / menus.html · .segmented (--fill) com role="radiogroup" e botões .btn--compact com aria-checked (L41)
import type { KeyboardEvent } from "react";
import { Icon } from "./Icon";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  icon?: string;
}

interface SegmentedProps<T extends string> {
  /** Rótulo visível do grupo (fica acima, ligado por `aria-labelledby`). */
  label: string;
  options: ReadonlyArray<SegmentedOption<T>>;
  value: T;
  onChange: (value: T) => void;
  /** Divide a largura igualmente (dentro de menus). */
  fill?: boolean;
  /** Id de um rótulo já existente; sem ele, o grupo usa `aria-label`. */
  labelledBy?: string;
}

/** Uma opção entre poucas. Setas trocam a opção; só a escolhida entra no Tab. */
export function Segmented<T extends string>({ label, options, value, onChange, fill, labelledBy }: SegmentedProps<T>) {
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    e.stopPropagation();
    const i = options.findIndex((o) => o.value === value);
    const next = options[(i + step + options.length) % options.length];
    onChange(next.value);
    e.currentTarget.querySelector<HTMLElement>(`[data-value="${next.value}"]`)?.focus();
  };
  return (
    <div
      className={fill ? "segmented segmented--fill" : "segmented"}
      role="radiogroup"
      aria-labelledby={labelledBy}
      aria-label={labelledBy ? undefined : label}
      onKeyDown={onKeyDown}
    >
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className="btn btn--compact"
          role="radio"
          aria-checked={o.value === value}
          data-value={o.value}
          tabIndex={o.value === value ? 0 : -1}
          onClick={() => onChange(o.value)}
        >
          {o.icon && <Icon name={o.icon} size="sm" />}
          {o.label}
        </button>
      ))}
    </div>
  );
}

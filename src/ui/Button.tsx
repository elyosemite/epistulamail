// DS: botoes.html · .btn, .btn--primary, .btn--subtle, .btn--link, .btn--icon, .btn--compact, .btn--block
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Icon } from "./Icon";
import { Spinner } from "./Spinner";

type Appearance = "default" | "primary" | "subtle" | "danger" | "warning" | "link";

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  appearance?: Appearance;
  icon?: string;
  compact?: boolean;
  block?: boolean;
  loading?: boolean;
  children?: ReactNode;
}

export function Button({ appearance = "default", icon, compact, block, loading, className, children, type = "button", disabled, ...rest }: ButtonProps) {
  const cls = [
    "btn",
    appearance !== "default" && `btn--${appearance}`,
    compact && "btn--compact",
    block && "btn--block",
    className,
  ].filter(Boolean).join(" ");
  return (
    <button type={type} className={cls} disabled={disabled || loading} {...rest}>
      {loading ? <Spinner /> : icon && <Icon name={icon} />}
      {children}
    </button>
  );
}

interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  icon: string;
  label: string;
  compact?: boolean;
}

/** Botão de ícone: 32 × 32, sempre com aria-label e tooltip (botoes.html, menus.html). */
export function IconButton({ icon, label, compact, className, type = "button", ...rest }: IconButtonProps) {
  const cls = ["btn btn--subtle btn--icon", compact && "btn--compact", className].filter(Boolean).join(" ");
  return (
    <button type={type} className={cls} aria-label={label} data-tip={label} {...rest}>
      <Icon name={icon} />
    </button>
  );
}

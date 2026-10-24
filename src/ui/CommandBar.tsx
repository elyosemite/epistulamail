// DS: header.html "Barra de comandos" · .commandbar (área .app-shell__commands), .commandbar__group, .commandbar__label,
//     .toolbar__sep; botoes.html · .btn--primary, .btn--subtle, .btn-split
import { forwardRef, type ReactNode } from "react";
import { Icon } from "./Icon";

export function CommandBar({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="commandbar app-shell__commands" role="toolbar" aria-label={label}>
      {children}
    </div>
  );
}

export const CommandGroup = ({ children }: { children: ReactNode }) => <div className="commandbar__group">{children}</div>;
export const CommandSeparator = () => <span className="toolbar__sep" role="separator" />;

interface CommandButtonProps {
  icon: string;
  label: string;
  /** Tooltip com o atalho, ex. "Arquivar (E)". */
  tip: string;
  primary?: boolean;
  disabled?: boolean;
  /** Abre um menu: mostra o chevron e `aria-haspopup`. */
  menu?: boolean;
  expanded?: boolean;
  onClick: (button: HTMLButtonElement) => void;
}

/** Botão de comando com ícone e rótulo (o rótulo some em telas estreitas; o nome acessível fica). */
export const CommandButton = forwardRef<HTMLButtonElement, CommandButtonProps>(function CommandButton(
  { icon, label, tip, primary, disabled, menu, expanded, onClick },
  ref,
) {
  return (
    <button
      ref={ref}
      type="button"
      className={primary ? "btn btn--primary" : "btn btn--subtle"}
      aria-label={label}
      aria-haspopup={menu ? "menu" : undefined}
      aria-expanded={menu ? !!expanded : undefined}
      data-tip={tip}
      disabled={disabled}
      onClick={(e) => onClick(e.currentTarget)}
    >
      <Icon name={icon} />
      {primary ? label : <span className="commandbar__label">{label}</span>}
      {menu && <Icon name="chevron-down" size="sm" />}
    </button>
  );
});

interface CommandSplitProps extends Omit<CommandButtonProps, "menu" | "expanded" | "primary"> {
  menuLabel: string;
  expanded: boolean;
  onMenu: (button: HTMLButtonElement) => void;
}

/** Ação principal + menu ao lado (`.btn-split`), como "Responder a todos ▾". */
export function CommandSplit({ icon, label, tip, disabled, onClick, menuLabel, expanded, onMenu }: CommandSplitProps) {
  return (
    <div className="btn-split">
      <CommandButton icon={icon} label={label} tip={tip} disabled={disabled} onClick={onClick} />
      <button
        type="button"
        className="btn btn--subtle"
        aria-label={menuLabel}
        aria-haspopup="menu"
        aria-expanded={expanded}
        data-tip={menuLabel}
        disabled={disabled}
        onClick={(e) => onMenu(e.currentTarget)}
      >
        <Icon name="chevron-down" size="sm" />
      </button>
    </div>
  );
}

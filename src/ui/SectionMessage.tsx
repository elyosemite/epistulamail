// DS: mensagens.html · .section-message e variantes --warning, --error, --success, --discovery
import type { ReactNode } from "react";
import { Icon } from "./Icon";

type Appearance = "information" | "warning" | "error" | "success" | "discovery";

const ICON: Record<Appearance, string> = {
  information: "info",
  warning: "warning",
  error: "error",
  success: "success",
  discovery: "sparkle",
};

interface Action {
  label: string;
  onClick: () => void;
}

interface SectionMessageProps {
  appearance?: Appearance;
  title: string;
  children?: ReactNode;
  actions?: Action[];
}

export function SectionMessage({ appearance = "information", title, children, actions }: SectionMessageProps) {
  const cls = ["section-message", appearance !== "information" && `section-message--${appearance}`].filter(Boolean).join(" ");
  return (
    <div className={cls} role={appearance === "error" ? "alert" : "note"}>
      <Icon name={ICON[appearance]} />
      <div>
        <p className="section-message__title">{title}</p>
        {children && <p>{children}</p>}
        {actions && actions.length > 0 && (
          <div className="section-message__actions">
            {actions.map((a) => (
              <button key={a.label} type="button" onClick={a.onClick}>{a.label}</button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

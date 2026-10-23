// DS: mensagens.html · .banner, .banner--warning, .banner--error, .banner--announcement
import type { ReactNode } from "react";
import { Icon } from "./Icon";

type Appearance = "warning" | "error" | "announcement";
const ICON: Record<Appearance, string> = { warning: "warning", error: "error", announcement: "info" };

export function Banner({ appearance, children }: { appearance: Appearance; children: ReactNode }) {
  return (
    <div className={`banner banner--${appearance}`} role={appearance === "error" ? "alert" : "status"}>
      <Icon name={ICON[appearance]} />
      <span>{children}</span>
    </div>
  );
}

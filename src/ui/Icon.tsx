// DS: ícones do sprite de design-system/docs.js (lacuna L2), classes .i de components.css.
import { sprite } from "virtual:ds-icons";

export function IconSprite() {
  return <div hidden dangerouslySetInnerHTML={{ __html: sprite }} />;
}

export type IconSize = "sm" | "md" | "lg" | "xl";

export function Icon({ name, size = "md", className }: { name: string; size?: IconSize; className?: string }) {
  const cls = ["i", size !== "md" && `i--${size}`, className].filter(Boolean).join(" ");
  return (
    <svg className={cls} aria-hidden="true">
      <use href={`#i-${name}`} />
    </svg>
  );
}

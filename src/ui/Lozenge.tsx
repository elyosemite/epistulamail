// DS: identidade.html · .lozenge e variantes --success, --removed, --inprogress, --new, --moved, --bold
type Appearance = "default" | "success" | "removed" | "inprogress" | "new" | "moved";

export function Lozenge({ appearance = "default", bold, children }: { appearance?: Appearance; bold?: boolean; children: string }) {
  const cls = ["lozenge", appearance !== "default" && `lozenge--${appearance}`, bold && "lozenge--bold"].filter(Boolean).join(" ");
  return <span className={cls}>{children}</span>;
}

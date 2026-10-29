// DS: lozenge.html · .lozenge e variantes --success, --removed, --inprogress, --new, --moved, --bold;
// menus.html · --overdue (prazo vencido da sinalização)
type Appearance = "default" | "success" | "removed" | "inprogress" | "new" | "moved" | "overdue";

export function Lozenge({ appearance = "default", bold, children }: { appearance?: Appearance; bold?: boolean; children: string }) {
  const cls = ["lozenge", appearance !== "default" && `lozenge--${appearance}`, bold && "lozenge--bold"].filter(Boolean).join(" ");
  return <span className={cls}>{children}</span>;
}

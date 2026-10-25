// DS: threads.html · .toolbar, .toolbar.is-selecting (com seleção na lista), .toolbar__end
import type { ReactNode } from "react";

export function Toolbar({ children, end, label, selecting }: { children?: ReactNode; end?: ReactNode; label: string; selecting?: boolean }) {
  return (
    <div className={selecting ? "toolbar is-selecting" : "toolbar"} role="toolbar" aria-label={label}>
      {children}
      {end && <div className="toolbar__end">{end}</div>}
    </div>
  );
}

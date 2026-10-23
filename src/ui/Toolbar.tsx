// DS: threads.html · .toolbar, .toolbar__end
import type { ReactNode } from "react";

export function Toolbar({ children, end, label }: { children?: ReactNode; end?: ReactNode; label: string }) {
  return (
    <div className="toolbar" role="toolbar" aria-label={label}>
      {children}
      {end && <div className="toolbar__end">{end}</div>}
    </div>
  );
}

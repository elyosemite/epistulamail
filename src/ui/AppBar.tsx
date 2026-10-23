// DS: header.html · .appbar, .appbar__brand com .wordmark.wordmark--sm, .appbar__actions
// DS: espacamento.html · .app-shell__header
import type { ReactNode } from "react";
import { Icon } from "./Icon";

export function AppBar({ actions }: { actions?: ReactNode }) {
  return (
    <header className="appbar app-shell__header">
      <span className="appbar__brand">
        <span className="wordmark wordmark--sm">
          <span className="wordmark__glyph" aria-hidden="true"><Icon name="mail" /></span>
          Epistula
        </span>
      </span>
      {actions && <div className="appbar__actions">{actions}</div>}
    </header>
  );
}

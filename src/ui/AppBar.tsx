// DS: header.html · .appbar, .appbar__brand com .wordmark.wordmark--sm, .appbar__actions
// DS: espacamento.html · .app-shell__header; .appbar--shell (subgrid) com .appbar__search sobre a lista
import type { ReactNode } from "react";
import { Icon } from "./Icon";

/** Com `search`, o header usa as colunas da casca (`.appbar--shell`): busca sobre a lista. */
export function AppBar({ actions, search }: { actions?: ReactNode; search?: ReactNode }) {
  return (
    <header className={search ? "appbar appbar--shell app-shell__header" : "appbar app-shell__header"}>
      <span className="appbar__brand">
        <span className="wordmark wordmark--sm">
          <span className="wordmark__glyph" aria-hidden="true"><Icon name="mail" /></span>
          Epistula
        </span>
      </span>
      {search && <div className="appbar__search">{search}</div>}
      {actions && <div className="appbar__actions">{actions}</div>}
    </header>
  );
}

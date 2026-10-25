// DS: espacamento.html "Em blocos" · .app-shell__block.app-shell__block--nav (subgrid com as pastas e a lista)
import type { ReactNode } from "react";

/** Bloco com a coluna de pastas e a coluna da lista de e-mails. */
export function NavigationBlock({ children }: { children: ReactNode }) {
  return <div className="app-shell__block app-shell__block--nav">{children}</div>;
}

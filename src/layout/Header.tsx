// DS: espacamento.html "Em blocos" · .app-shell__block.app-shell__block--header com .appbar.appbar--shell (subgrid:
//     marca sobre as pastas, busca + configurações sobre a lista, conta sobre a leitura) e a .commandbar como
//     segunda linha (D1)
import type { ReactNode } from "react";

interface HeaderProps {
  brand: ReactNode;
  search?: ReactNode;
  account?: ReactNode;
  /** Barra de comandos (segunda linha do bloco). */
  commands?: ReactNode;
}

/** Bloco do header: agrupa marca, busca, configurações, conta ativa e comandos sobre uma superfície. */
export function Header({ brand, search, account, commands }: HeaderProps) {
  return (
    <div className="app-shell__block app-shell__block--header">
      <header className="appbar appbar--shell">
        {brand}
        {search}
        {account && <div className="appbar__actions">{account}</div>}
      </header>
      {commands}
    </div>
  );
}

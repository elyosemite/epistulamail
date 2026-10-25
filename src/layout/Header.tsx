// DS: espacamento.html "Em blocos" · .app-shell__block.app-shell__block--header com .appbar.appbar--shell (subgrid:
//     marca sobre as pastas, busca sobre a lista e, sobre a leitura, o fim .appbar__end com os botões das colunas e a
//     foto da conta na extrema direita, US7) e a .commandbar como segunda linha (D1 da US5)
import type { ReactNode } from "react";

interface HeaderProps {
  brand: ReactNode;
  search?: ReactNode;
  /** Fim do header: botões das colunas e a foto da conta (menu da conta). */
  end?: ReactNode;
  /** Barra de comandos (segunda linha do bloco). */
  commands?: ReactNode;
}

/** Bloco do header: marca, busca, colunas, conta ativa e comandos sobre uma superfície. */
export function Header({ brand, search, end, commands }: HeaderProps) {
  return (
    <div className="app-shell__block app-shell__block--header">
      <header className="appbar appbar--shell">
        {brand}
        {search}
        {end && <div className="appbar__actions appbar__end">{end}</div>}
      </header>
      {commands}
    </div>
  );
}

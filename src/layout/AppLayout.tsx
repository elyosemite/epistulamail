// DS: espacamento.html "Em blocos" · .app-shell.app-shell--with-rail.app-shell--blocks (fundo --app-background,
//     gaps de 8px), .has-reader; a largura da lista vem de --app-shell-list; "Colunas recolhíveis" ·
//     .app-shell--no-folders e .app-shell--no-reader (US7)
import type { CSSProperties, ReactNode } from "react";

interface AppLayoutProps {
  /** Largura da lista em pixels (divisor). */
  listWidth: number;
  /** Abaixo de 960px a leitura ocupa o lugar do bloco de navegação. */
  hasReader: boolean;
  /** Coluna de pastas visível (Ctrl+B). */
  showFolders?: boolean;
  /** Coluna de leitura visível (Ctrl+Alt+B). */
  showReader?: boolean;
  children: ReactNode;
}

/** A casca em blocos: coluna de contas, bloco do header, bloco de navegação e bloco de conteúdo. */
export function AppLayout({ listWidth, hasReader, showFolders = true, showReader = true, children }: AppLayoutProps) {
  const cls = [
    "app-shell app-shell--with-rail app-shell--blocks",
    hasReader && "has-reader",
    !showFolders && "app-shell--no-folders",
    !showReader && "app-shell--no-reader",
  ].filter(Boolean).join(" ");
  return (
    <div className={cls} style={{ "--app-shell-list": `${listWidth}px` } as CSSProperties}>
      {children}
    </div>
  );
}

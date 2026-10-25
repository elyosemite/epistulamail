// DS: espacamento.html "Em blocos" · .app-shell.app-shell--with-rail.app-shell--blocks (fundo --app-background,
//     gaps de 8px), .has-reader; a largura da lista vem de --app-shell-list
import type { CSSProperties, ReactNode } from "react";

interface AppLayoutProps {
  /** Largura da lista em pixels (divisor). */
  listWidth: number;
  /** Abaixo de 960px a leitura ocupa o lugar do bloco de navegação. */
  hasReader: boolean;
  children: ReactNode;
}

/** A casca em blocos: coluna de contas, bloco do header, bloco de navegação e bloco de conteúdo. */
export function AppLayout({ listWidth, hasReader, children }: AppLayoutProps) {
  const cls = ["app-shell app-shell--with-rail app-shell--blocks", hasReader && "has-reader"].filter(Boolean).join(" ");
  return (
    <div className={cls} style={{ "--app-shell-list": `${listWidth}px` } as CSSProperties}>
      {children}
    </div>
  );
}

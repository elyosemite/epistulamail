// DS: espacamento.html "Em blocos" · .app-shell__reader.app-shell__block (bloco de conteúdo); a leitura usa
//     .reader--fluid e acompanha a largura do bloco
import type { ReactNode } from "react";

/** Bloco de conteúdo: a mensagem aberta, a resposta ou o e-mail novo. */
export function ContentColumn({ children }: { children: ReactNode }) {
  return (
    <section className="app-shell__reader app-shell__block" aria-label="Mensagem">
      {children}
    </section>
  );
}

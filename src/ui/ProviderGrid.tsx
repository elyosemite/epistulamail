// DS: selecao.html "Grade de provedores" · .provider-grid, .provider-card (__icon, __text, __name, __desc) + .lozenge "Em breve"
import { useId } from "react";
import { Icon } from "./Icon";
import { Lozenge } from "./Lozenge";

export interface ProviderOption {
  id: string;
  name: string;
  description: string;
  available: boolean;
  icon: string;
}

/** `wide`: cada cartão ocupa a linha inteira (o "Abrir arquivo de e-mail", US15). */
export function ProviderGrid({ providers, wide, onPick }: { providers: ProviderOption[]; wide?: boolean; onPick: (provider: ProviderOption) => void }) {
  const soonId = useId();
  return (
    <>
      <ul className="provider-grid" role="list">
        {providers.map((p) => (
          <li key={p.id} style={wide ? { gridColumn: "1 / -1" } : undefined}>
            <button
              className="provider-card"
              type="button"
              disabled={!p.available}
              aria-describedby={p.available ? undefined : soonId}
              onClick={() => onPick(p)}
            >
              <span className="provider-card__icon" aria-hidden="true"><Icon name={p.icon} /></span>
              <span className="provider-card__text">
                <span className="provider-card__name">{p.name}{!p.available && <Lozenge>Em breve</Lozenge>}</span>
                <span className="provider-card__desc">{p.description}</span>
              </span>
            </button>
          </li>
        ))}
      </ul>
      <span className="visually-hidden" id={soonId}>Ainda não disponível no Epistula.</span>
    </>
  );
}

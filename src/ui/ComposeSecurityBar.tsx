// DS: criptografia.html "Escrever com criptografia" · .compose__security com botões aria-pressed e .compose__security-tech (L68),
//     com "Trocar" (.btn--link) quando a conta tem OpenPGP e S/MIME (US14)
import type { CryptoTechnology } from "../lib/tauri";
import { Icon } from "./Icon";

const TECH: Record<CryptoTechnology, string> = { openpgp: "OpenPGP", smime: "S/MIME" };

interface ComposeSecurityBarProps {
  encrypt: boolean;
  sign: boolean;
  technology: CryptoTechnology;
  /** Só quando a conta tem as duas tecnologias. */
  onSwitch?: () => void;
  /** Assinar não desliga enquanto a mensagem vai criptografada (é sempre assinada). */
  onEncrypt: (on: boolean) => void;
  onSign: (on: boolean) => void;
}

export function ComposeSecurityBar({ encrypt, sign, technology, onSwitch, onEncrypt, onSign }: ComposeSecurityBarProps) {
  return (
    <div className="compose__security" role="group" aria-label="Segurança">
      <button className="btn btn--subtle btn--compact" type="button" aria-pressed={encrypt} onClick={() => onEncrypt(!encrypt)}>
        <Icon name={encrypt ? "lock" : "lock-open"} />
        Criptografar
      </button>
      <button
        className="btn btn--subtle btn--compact"
        type="button"
        aria-pressed={sign}
        disabled={encrypt}
        data-tip={encrypt ? "Mensagens criptografadas são sempre assinadas" : undefined}
        onClick={() => onSign(!sign)}
      >
        <Icon name="signature" />
        Assinar
      </button>
      <span className="compose__security-tech">
        {TECH[technology]}
        {onSwitch && (
          <button className="btn btn--link" type="button" style={{ marginLeft: "var(--space-075)" }} aria-label={`Trocar para ${TECH[technology === "openpgp" ? "smime" : "openpgp"]}`} onClick={onSwitch}>
            Trocar
          </button>
        )}
      </span>
    </div>
  );
}

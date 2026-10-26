// DS: criptografia.html "Escrever com criptografia" · .compose__security com botões aria-pressed e .compose__security-tech (L68)
import { Icon } from "./Icon";

interface ComposeSecurityBarProps {
  encrypt: boolean;
  sign: boolean;
  /** Assinar não desliga enquanto a mensagem vai criptografada (é sempre assinada). */
  onEncrypt: (on: boolean) => void;
  onSign: (on: boolean) => void;
}

export function ComposeSecurityBar({ encrypt, sign, onEncrypt, onSign }: ComposeSecurityBarProps) {
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
      <span className="compose__security-tech">OpenPGP</span>
    </div>
  );
}

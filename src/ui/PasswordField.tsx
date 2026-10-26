// DS: campos.html "Senha" · .field, .field__label, .input.input--password (L54), .field__help (--error); botoes.html .btn--icon.btn--compact
import { useId, useState, type InputHTMLAttributes, type ReactNode } from "react";
import { Icon } from "./Icon";

interface PasswordFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "id" | "type"> {
  label: string;
  error?: string;
  /** Texto de ajuda abaixo do campo, quando não há erro. */
  help?: ReactNode;
}

/** Campo de senha com o botão de mostrar no fim (US10). O ícone mostra o que o clique faz. */
export function PasswordField({ label, error, help, ...input }: PasswordFieldProps) {
  const id = useId();
  const helpId = `${id}-help`;
  const [shown, setShown] = useState(false);
  const toggle = shown ? "Esconder a senha" : "Mostrar a senha";
  return (
    <div className="field">
      <label className="field__label" htmlFor={id}>{label}</label>
      <div className={["input input--password", error && "is-invalid"].filter(Boolean).join(" ")}>
        <input
          id={id}
          type={shown ? "text" : "password"}
          spellCheck={false}
          aria-invalid={error ? true : undefined}
          aria-describedby={error || help ? helpId : undefined}
          {...input}
        />
        <button type="button" className="btn btn--subtle btn--icon btn--compact" aria-label={toggle} data-tip={toggle} onClick={() => setShown((s) => !s)}>
          <Icon name={shown ? "eye-off" : "eye"} />
        </button>
      </div>
      {error ? (
        <span className="field__help field__help--error" id={helpId} role="alert">
          <Icon name="error" size="sm" />
          {error}
        </span>
      ) : (
        help && <span className="field__help" id={helpId}>{help}</span>
      )}
    </div>
  );
}

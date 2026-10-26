// DS: criptografia.html · .pem-box com .pem-box__copy (L69): texto em armadura para copiar (chave pública, pedido de certificado)
import { useState } from "react";
import { IconButton } from "./Button";

export function PemBox({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <pre className="pem-box" aria-label={label}>
      {text}
      <IconButton
        icon={copied ? "check" : "copy"}
        label={copied ? "Copiado" : "Copiar"}
        compact
        className="pem-box__copy"
        onClick={() => {
          void navigator.clipboard.writeText(text).then(() => setCopied(true)).catch(() => undefined);
        }}
      />
    </pre>
  );
}

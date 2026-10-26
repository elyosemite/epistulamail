// DS: modais.html · .confirm-check (L57) com .checkbox: confirmação de texto longo dentro de um modal
import type { ReactNode } from "react";

export function ConfirmCheck({ checked, onChange, children }: { checked: boolean; onChange: (checked: boolean) => void; children: ReactNode }) {
  return (
    <label className="confirm-check">
      <input type="checkbox" className="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span>{children}</span>
    </label>
  );
}

// DS: header.html · .appbar__search sobre a coluna da lista, com .input--search
import { forwardRef } from "react";
import { SearchField } from "../ui/SearchField";

interface HeaderSearchProps {
  value: string;
  placeholder: string;
  /** Com as configurações abertas, a busca não responde (D2). */
  inert?: boolean;
  onChange: (value: string) => void;
  onEscape: () => void;
}

export const HeaderSearch = forwardRef<HTMLInputElement, HeaderSearchProps>(function HeaderSearch(
  { value, placeholder, inert, onChange, onEscape },
  ref,
) {
  return (
    <div className="appbar__search">
      <SearchField
        ref={ref}
        value={value}
        placeholder={placeholder}
        label="Buscar e-mails nas mensagens salvas"
        shortcut="/"
        inert={inert}
        onChange={onChange}
        onEscape={onEscape}
      />
    </div>
  );
});

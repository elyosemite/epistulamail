// DS: header.html · .appbar__search com .input.input--search (ícone, campo, kbd do atalho, botão de limpar)
import { forwardRef } from "react";
import { IconButton } from "./Button";
import { Icon } from "./Icon";

interface SearchFieldProps {
  value: string;
  placeholder: string;
  label: string;
  shortcut?: string;
  onChange: (value: string) => void;
  onEscape: () => void;
}

export const SearchField = forwardRef<HTMLInputElement, SearchFieldProps>(function SearchField(
  { value, placeholder, label, shortcut, onChange, onEscape },
  ref,
) {
  return (
    <div className="input input--search">
      <Icon name="search" />
      <input
        ref={ref}
        type="search"
        value={value}
        placeholder={placeholder}
        aria-label={label}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onEscape();
          }
        }}
      />
      {value ? <IconButton icon="x" label="Limpar busca" compact onClick={onEscape} /> : shortcut && <kbd>{shortcut}</kbd>}
    </div>
  );
});

// DS: modais.html "Seletor de pastas" · .modal, .folder-picker (role="listbox") com .nav-item[aria-selected],
//     .modal__foot--split; header.html · .input--search
import { useMemo, useState, type KeyboardEvent } from "react";
import { Button } from "./Button";
import { TextField } from "./Field";
import { Icon } from "./Icon";
import { Modal } from "./Modal";

export interface PickerFolder {
  id: string;
  name: string;
  icon: string;
}

interface FolderPickerProps {
  folders: PickerFolder[];
  onMove: (folderId: string) => void;
  onCreate: (name: string) => Promise<void>;
  onClose: () => void;
}

export function FolderPicker({ folders, onMove, onCreate, onClose }: FolderPickerProps) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return folders.filter((f) => f.name.toLowerCase().includes(q));
  }, [folders, query]);

  const onListKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const i = shown.findIndex((f) => f.id === selected);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = shown[Math.max(0, Math.min(shown.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
      if (next) setSelected(next.id);
    } else if (e.key === "Enter" && selected) {
      e.preventDefault();
      onMove(selected);
    }
  };

  const create = async () => {
    if (!name.trim()) return setError("Dê um nome para a pasta.");
    setBusy(true);
    try {
      await onCreate(name.trim());
    } catch (e) {
      setError(e && typeof e === "object" && "message" in e ? String(e.message) : "Não foi possível criar a pasta.");
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Mover para"
      initialFocus="input"
      splitFooter
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" icon="folder-plus" onClick={() => setCreating(true)}>Nova pasta</Button>
          <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" disabled={!selected} onClick={() => selected && onMove(selected)}>Mover</Button>
        </>
      }
    >
      {creating && (
        <form
          style={{ display: "grid", gap: "var(--space-100)", marginBottom: "var(--space-150)" }}
          onSubmit={(e) => {
            e.preventDefault();
            void create();
          }}
        >
          <TextField label="Nome da nova pasta" value={name} maxLength={100} error={error} onChange={(e) => { setName(e.target.value); setError(undefined); }} />
          <div style={{ display: "flex", gap: "var(--space-100)", justifyContent: "flex-end" }}>
            <Button appearance="subtle" compact onClick={() => setCreating(false)}>Cancelar</Button>
            <Button appearance="primary" compact type="submit" loading={busy}>Criar e mover</Button>
          </div>
        </form>
      )}
      <div className="input input--search">
        <Icon name="search" />
        <input placeholder="Buscar pasta" aria-label="Buscar pasta" value={query} onChange={(e) => setQuery(e.target.value)} />
      </div>
      <div className="folder-picker" role="listbox" aria-label="Pastas" tabIndex={0} onKeyDown={onListKey}>
        {shown.map((f) => (
          <div
            key={f.id}
            className="nav-item"
            role="option"
            aria-selected={f.id === selected}
            onClick={() => setSelected(f.id)}
            onDoubleClick={() => onMove(f.id)}
          >
            <Icon name={f.icon} />
            <span className="nav-item__text">{f.name}</span>
          </div>
        ))}
        {shown.length === 0 && <div className="menu__empty">Nenhuma pasta com esse nome.</div>}
      </div>
    </Modal>
  );
}

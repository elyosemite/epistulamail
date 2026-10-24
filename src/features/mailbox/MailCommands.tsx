// Composição de DS: header.html (.commandbar), menus.html (.menu "Mover para" com .menu__search, .menu__form e
// .menu__empty; menu de resposta; menu "Sinalizar" com .menu-item__meta), modais.html (.folder-picker)
import { useEffect, useMemo, useRef, useState } from "react";
import { dueFor, FOLLOWUP_CHOICES, shortDate } from "../../lib/followup";
import { api, type Account, type DraftKind, type Folder, type FolderRole, type FollowupAction, type MessageHeader } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { CommandBar, CommandButton, CommandGroup, CommandSeparator, CommandSplit } from "../../ui/CommandBar";
import { FolderPicker } from "../../ui/FolderPicker";
import { Icon } from "../../ui/Icon";
import { anchorBelow, MenuHeading, MenuItem, MenuPanel, MenuSeparator, type Anchor } from "../../ui/Menu";

const FOLDER_ICON: Record<FolderRole, string> = {
  inbox: "inbox",
  starred: "star",
  sent: "send",
  drafts: "file",
  all: "archive",
  spam: "spam",
  trash: "trash",
  custom: "folder",
};

export const folderIcon = (role: FolderRole) => FOLDER_ICON[role];

type OpenMenu = { kind: "move" | "reply" | "flag"; anchor: Anchor } | null;

interface MailCommandsProps {
  account: Account;
  folders: Folder[];
  /** Pasta da mensagem selecionada (na busca, a pasta de cada resultado). */
  sourceFolderId: string | null;
  selected: MessageHeader | null;
  /** Conta sem acesso ao servidor: só "Novo e-mail" funciona. */
  readOnly: boolean;
  onNew: () => void;
  onDelete: () => void;
  onArchive: () => void;
  onMove: (folderId: string) => void;
  onCreateAndMove: (name: string) => Promise<void>;
  onReply: (kind: Exclude<DraftKind, "new">) => void;
  onToggleRead: () => void;
  onFollowup: (change: FollowupAction) => void;
}

/** Atalhos de teclado da barra (fora de campos de texto). */
function isTyping(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));
}

export function MailCommands(props: MailCommandsProps) {
  const { account, folders, sourceFolderId, selected, readOnly, onNew, onDelete, onArchive, onMove, onCreateAndMove, onReply, onToggleRead, onFollowup } = props;
  const [menu, setMenu] = useState<OpenMenu>(null);
  const [picker, setPicker] = useState(false);
  const [recent, setRecent] = useState<Folder[]>([]);
  const [query, setQuery] = useState("");
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const moveButton = useRef<HTMLButtonElement>(null);
  const disabled = !selected || readOnly;
  const source = folders.find((f) => f.id === sourceFolderId);

  const targets = useMemo(() => folders.filter((f) => f.id !== sourceFolderId && f.role !== "starred"), [folders, sourceFolderId]);

  const openMove = (anchor: Anchor) => {
    setQuery("");
    setNewName("");
    setCreateError(null);
    setMenu({ kind: "move", anchor });
    api.recentFolders(account.id).then(setRecent).catch(() => setRecent([]));
  };

  // Atalhos: N, Delete/#, E, V, R, Shift+R, Shift+F, Insert.
  const latest = useRef(props);
  latest.current = props;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target) || document.querySelector(".blanket")) return;
      const p = latest.current;
      const can = !!p.selected && !p.readOnly;
      const run = (fn: () => void) => {
        e.preventDefault();
        fn();
      };
      if (e.key === "n" || e.key === "N") return e.shiftKey ? undefined : run(p.onNew);
      if (!can) return;
      if (e.key === "Delete" || e.key === "#") run(p.onDelete);
      else if (e.key === "e" && !e.shiftKey) run(p.onArchive);
      else if (e.key === "v" && !e.shiftKey && moveButton.current) run(() => openMove(anchorBelow(moveButton.current!)));
      else if (e.key === "r" && !e.shiftKey) run(() => p.onReply("reply"));
      else if (e.key === "R" && e.shiftKey) run(() => p.onReply("reply_all"));
      else if (e.key === "F" && e.shiftKey) run(() => p.onReply("forward"));
      else if (e.key === "Insert") run(() => p.onFollowup(p.selected!.flags.flagged ? { action: "clear" } : { action: "set", due: null }));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // Registrado uma vez: os valores atuais vêm de `latest`, e openMove só usa setters estáveis.
  }, []);

  const close = () => setMenu(null);
  const chooseFolder = (id: string) => {
    close();
    setPicker(false);
    onMove(id);
  };

  const create = async () => {
    const name = newName.trim();
    if (!name) return;
    setCreating(true);
    try {
      await onCreateAndMove(name);
      close();
    } catch (e) {
      setCreateError(e && typeof e === "object" && "message" in e ? String(e.message) : "Não foi possível criar a pasta.");
    } finally {
      setCreating(false);
    }
  };

  const q = query.trim().toLowerCase();
  const matches = q ? targets.filter((f) => f.name.toLowerCase().includes(q)) : recent.filter((f) => f.id !== sourceFolderId && f.role !== "starred");
  const seen = selected?.flags.seen ?? true;
  const flagged = selected?.flags.flagged ?? false;
  const today = new Date();

  return (
    <>
      <CommandBar label="Ações da mensagem">
        <CommandButton primary icon="pencil" label="Novo e-mail" tip="Novo e-mail (N)" onClick={onNew} />
        <CommandSeparator />
        <CommandGroup>
          <CommandButton icon="trash" label="Excluir" tip={source?.role === "trash" ? "Excluir para sempre (Delete)" : "Excluir (Delete)"} disabled={disabled} onClick={onDelete} />
          <CommandButton icon="archive" label="Arquivar" tip="Arquivar (E)" disabled={disabled || source?.role === "all"} onClick={onArchive} />
          <CommandButton
            ref={moveButton}
            icon="folder-move"
            label="Mover para"
            tip="Mover para (V)"
            menu
            expanded={menu?.kind === "move"}
            disabled={disabled}
            onClick={(b) => openMove(anchorBelow(b))}
          />
          <CommandSplit
            icon="reply-all"
            label="Responder a todos"
            tip="Responder a todos (Shift+R)"
            menuLabel="Responder ou encaminhar"
            expanded={menu?.kind === "reply"}
            disabled={disabled}
            onClick={() => onReply("reply_all")}
            onMenu={(b) => setMenu({ kind: "reply", anchor: anchorBelow(b) })}
          />
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup>
          <CommandButton
            icon={seen ? "mail" : "mail-open"}
            label={seen ? "Marcar como não lida" : "Marcar como lida"}
            tip={seen ? "Marcar como não lida (Shift+U)" : "Marcar como lida (Shift+U)"}
            disabled={disabled}
            onClick={onToggleRead}
          />
          <CommandSplit
            icon="flag"
            label={flagged ? "Limpar sinalização" : "Sinalizar"}
            tip={flagged ? "Limpar sinalização (Insert)" : "Sinalizar (Insert)"}
            menuLabel="Prazo da sinalização"
            expanded={menu?.kind === "flag"}
            disabled={disabled}
            onClick={() => onFollowup(flagged ? { action: "clear" } : { action: "set", due: null })}
            onMenu={(b) => setMenu({ kind: "flag", anchor: anchorBelow(b) })}
          />
        </CommandGroup>
      </CommandBar>

      {menu?.kind === "move" && (
        <MenuPanel label="Mover para" anchor={menu.anchor} wide onClose={close}>
          <div className="menu__search">
            <div className="input input--search">
              <Icon name="search" />
              <input placeholder="Buscar pasta" aria-label="Buscar pasta" value={query} onChange={(e) => setQuery(e.target.value)} />
            </div>
          </div>
          <MenuSeparator />
          <MenuHeading>{q ? "Pastas" : "Mais recentes"}</MenuHeading>
          {matches.map((f) => (
            <MenuItem key={f.id} label={f.name} icon={folderIcon(f.role)} onSelect={() => chooseFolder(f.id)} />
          ))}
          {matches.length === 0 && <div className="menu__empty">{q ? "Nenhuma pasta com esse nome." : "Nenhuma pasta usada ainda."}</div>}
          <MenuSeparator />
          <MenuHeading>Criar nova pasta</MenuHeading>
          <form
            className="menu__form"
            onSubmit={(e) => {
              e.preventDefault();
              void create();
            }}
          >
            <div className={["input", createError && "is-invalid"].filter(Boolean).join(" ")}>
              <input
                placeholder="Nome da pasta"
                aria-label="Nome da nova pasta"
                maxLength={100}
                value={newName}
                aria-invalid={createError ? true : undefined}
                onChange={(e) => {
                  setNewName(e.target.value);
                  setCreateError(null);
                }}
              />
            </div>
            <Button appearance="primary" compact type="submit" loading={creating} disabled={!newName.trim()}>Salvar</Button>
          </form>
          {createError && <div className="menu__empty" role="alert">{createError}</div>}
          <MenuSeparator />
          <MenuItem label="Selecionar outra pasta…" icon="folder-move" onSelect={() => { close(); setPicker(true); }} />
        </MenuPanel>
      )}

      {menu?.kind === "reply" && (
        <MenuPanel label="Responder ou encaminhar" anchor={menu.anchor} onClose={close}>
          <MenuItem label="Responder" icon="reply" kbd="R" onSelect={() => { close(); onReply("reply"); }} />
          <MenuItem label="Responder a todos" icon="reply-all" kbd="⇧R" onSelect={() => { close(); onReply("reply_all"); }} />
          <MenuItem label="Encaminhar" icon="forward" kbd="⇧F" onSelect={() => { close(); onReply("forward"); }} />
        </MenuPanel>
      )}

      {menu?.kind === "flag" && (
        <MenuPanel label="Sinalizar" anchor={menu.anchor} wide onClose={close}>
          <MenuHeading>Sinalizar para</MenuHeading>
          {FOLLOWUP_CHOICES.map(({ choice, label }) => {
            const due = dueFor(choice, today);
            const current = selected?.followup && !selected.followup.completed && selected.flags.flagged ? selected.followup.due : undefined;
            return (
              <MenuItem
                key={choice}
                label={label}
                icon="flag"
                meta={due ? shortDate(due) : undefined}
                selected={current !== undefined && current === due}
                onSelect={() => { close(); onFollowup({ action: "set", due }); }}
              />
            );
          })}
          <MenuSeparator />
          <MenuItem label="Marcar como concluída" icon="check" onSelect={() => { close(); onFollowup({ action: "complete" }); }} />
          <MenuItem label="Limpar sinalização" icon="x" onSelect={() => { close(); onFollowup({ action: "clear" }); }} />
        </MenuPanel>
      )}

      {picker && (
        <FolderPicker
          folders={targets.map((f) => ({ id: f.id, name: f.name, icon: folderIcon(f.role) }))}
          onMove={chooseFolder}
          onCreate={async (name) => {
            await onCreateAndMove(name);
            setPicker(false);
          }}
          onClose={() => setPicker(false)}
        />
      )}
    </>
  );
}

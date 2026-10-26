// Composição de DS: header.html (.commandbar, segunda linha do bloco do header), menus.html (.menu "Mover para" com
// .menu__search, .menu__form e .menu__empty; menu de resposta; menu "Sinalizar" com .menu-item__meta), modais.html
// (.folder-picker)
import type { QuickIntent } from "../features/rules/RuleDialogs";
import { useEffect, useMemo, useRef, useState } from "react";
import { canReceive, folderIcon } from "../lib/folders";
import { dueFor, FOLLOWUP_CHOICES, shortDate } from "../lib/followup";
import { conversations, readActionFor } from "../lib/selection";
import { api, type Account, type DraftKind, type Folder, type FollowupAction, type MessageHeader } from "../lib/tauri";
import { Button } from "../ui/Button";
import { CommandButton, CommandGroup, CommandSeparator, CommandSplit, CommandToolbar } from "../ui/CommandBar";
import { FolderPicker } from "../ui/FolderPicker";
import { Icon } from "../ui/Icon";
import { anchorBelow, MenuHeading, MenuItem, MenuPanel, MenuSeparator, type Anchor } from "../ui/Menu";

type OpenMenu = { kind: "move" | "reply" | "flag" | "more"; anchor: Anchor } | null;

interface CommandBarProps {
  account: Account;
  folders: Folder[];
  /** Pasta de origem quando todas as mensagens vêm da mesma (fora da busca); `null` na busca. */
  sourceFolderId: string | null;
  /** Em quem os comandos agem: as marcadas ou, sem marcadas, a aberta (US5, CA2). */
  targets: MessageHeader[];
  /** Pasta de cada alvo (na busca, cada resultado tem a sua). */
  roleOf: (message: MessageHeader) => Folder["role"] | undefined;
  /** Conta sem acesso ao servidor: só "Novo e-mail" funciona. */
  readOnly: boolean;
  /** Arquivo local (US15): sem escrever, responder nem regras; o resto age só no banco. */
  local?: boolean;
  /** Configurações abertas: a barra não responde (D2). */
  inert: boolean;
  onNew: () => void;
  onDelete: () => void;
  onArchive: () => void;
  onMove: (folderId: string) => void;
  onCreateAndMove: (name: string) => Promise<void>;
  onReply: (kind: Exclude<DraftKind, "new">) => void;
  onToggleRead: () => void;
  onFollowup: (change: FollowupAction) => void;
  /** "Criar regra…" e os atalhos de uma ação (US12, CA5), a partir da mensagem aberta. */
  onRule: (intent: QuickIntent) => void;
}

/** Atalhos de teclado da barra (fora de campos de texto). */
function isTyping(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));
}

/** Barra de comandos: novo e-mail, excluir, arquivar, mover, responder, lida e sinalizar. */
export function CommandBar(props: CommandBarProps) {
  const { account, folders, sourceFolderId, targets, roleOf, readOnly, local, inert, onNew, onDelete, onArchive, onMove, onCreateAndMove, onReply, onToggleRead, onFollowup, onRule } = props;
  const [menu, setMenu] = useState<OpenMenu>(null);
  const [picker, setPicker] = useState(false);
  const [recent, setRecent] = useState<Folder[]>([]);
  const [query, setQuery] = useState("");
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const moveButton = useRef<HTMLButtonElement>(null);
  const disabled = targets.length === 0 || readOnly;
  const single = targets.length === 1 ? targets[0] : null;
  const roles = targets.map(roleOf);

  const destinations = useMemo(() => folders.filter((f) => canReceive(f, sourceFolderId)), [folders, sourceFolderId]);

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
      if (e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target) || document.querySelector(".blanket, .settings-sheet")) return;
      const p = latest.current;
      const can = p.targets.length > 0 && !p.readOnly;
      const one = p.targets.length === 1;
      const run = (fn: () => void) => {
        e.preventDefault();
        fn();
      };
      if (!p.local && (e.key === "n" || e.key === "N")) return e.shiftKey ? undefined : run(p.onNew);
      if (!can) return;
      if (p.local && ["r", "R", "F"].includes(e.key)) return;
      if (e.key === "Delete" || e.key === "#") run(p.onDelete);
      else if (e.key === "e" && !e.shiftKey) run(p.onArchive);
      else if (e.key === "v" && !e.shiftKey && moveButton.current) run(() => openMove(anchorBelow(moveButton.current!)));
      else if (e.key === "r" && !e.shiftKey && one) run(() => p.onReply("reply"));
      else if (e.key === "R" && e.shiftKey && one) run(() => p.onReply("reply_all"));
      else if (e.key === "F" && e.shiftKey && one) run(() => p.onReply("forward"));
      else if (e.key === "Insert") run(() => p.onFollowup(p.targets.every((m) => m.flags.flagged) ? { action: "clear" } : { action: "set", due: null }));
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
  const matches = q ? destinations.filter((f) => f.name.toLowerCase().includes(q)) : recent.filter((f) => canReceive(f, sourceFolderId));
  const markSeen = readActionFor(targets).seen;
  const flagged = targets.length > 0 && targets.every((m) => m.flags.flagged);
  const today = new Date();
  const count = targets.length > 1 ? ` ${conversations(targets.length)}` : "";
  const moveLabel = targets.length > 1 ? `Mover ${conversations(targets.length)} para` : "Mover para";

  return (
    <>
      <CommandToolbar label={targets.length > 1 ? `Ações de ${conversations(targets.length)}` : "Ações da mensagem"} inert={inert}>
        {!local && (
          <>
            <CommandButton primary icon="pencil" label="Novo e-mail" tip="Novo e-mail (N)" onClick={onNew} />
            <CommandSeparator />
          </>
        )}
        <CommandGroup>
          <CommandButton
            icon="trash"
            label="Excluir"
            tip={roles.includes("trash") ? `Excluir para sempre${count} (Delete)` : `Excluir${count} (Delete)`}
            disabled={disabled}
            onClick={onDelete}
          />
          <CommandButton icon="archive" label="Arquivar" tip={`Arquivar${count} (E)`} disabled={disabled || roles.every((r) => r === "all")} onClick={onArchive} />
          <CommandButton
            ref={moveButton}
            icon="folder-move"
            label="Mover para"
            tip={`${moveLabel} (V)`}
            menu
            expanded={menu?.kind === "move"}
            disabled={disabled}
            onClick={(b) => openMove(anchorBelow(b))}
          />
          {!local && <CommandSplit
            icon="reply-all"
            label="Responder a todos"
            tip={single ? "Responder a todos (Shift+R)" : "Selecione uma conversa só para responder"}
            menuLabel="Responder ou encaminhar"
            expanded={menu?.kind === "reply"}
            disabled={disabled || !single}
            onClick={() => onReply("reply_all")}
            onMenu={(b) => setMenu({ kind: "reply", anchor: anchorBelow(b) })}
          />}
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup>
          <CommandButton
            icon={markSeen ? "mail-open" : "mail"}
            label={markSeen ? "Marcar como lida" : "Marcar como não lida"}
            tip={markSeen ? `Marcar${count} como lida (Shift+U)` : `Marcar${count} como não lida (Shift+U)`}
            disabled={disabled}
            onClick={onToggleRead}
          />
          <CommandSplit
            icon="flag"
            label={flagged ? "Limpar sinalização" : "Sinalizar"}
            tip={flagged ? `Limpar sinalização${count} (Insert)` : `Sinalizar${count} (Insert)`}
            menuLabel="Prazo da sinalização"
            expanded={menu?.kind === "flag"}
            disabled={disabled}
            onClick={() => onFollowup(flagged ? { action: "clear" } : { action: "set", due: null })}
            onMenu={(b) => setMenu({ kind: "flag", anchor: anchorBelow(b) })}
          />
          {!local && <CommandButton
            icon="more"
            label="Mais"
            tip={single ? "Mais ações" : "Selecione uma conversa só para criar uma regra"}
            menu
            expanded={menu?.kind === "more"}
            disabled={disabled || !single}
            onClick={(b) => setMenu({ kind: "more", anchor: anchorBelow(b) })}
          />}
        </CommandGroup>
      </CommandToolbar>

      {menu?.kind === "move" && (
        <MenuPanel label={moveLabel} anchor={menu.anchor} wide onClose={close}>
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

      {menu?.kind === "more" && single && (
        <MenuPanel label="Mais" anchor={menu.anchor} onClose={close}>
          <MenuItem label="Criar regra…" icon="filter" onSelect={() => { close(); onRule("rule"); }} />
          <MenuItem label={`Sempre mover de ${single.fromName || single.fromAddress} para…`} icon="folder-move" onSelect={() => { close(); onRule("always-move"); }} />
          <MenuItem label={`Excluir tudo de ${single.fromName || single.fromAddress}…`} icon="trash" onSelect={() => { close(); onRule("delete-all"); }} />
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
          <MenuHeading>{targets.length > 1 ? `Sinalizar ${conversations(targets.length)} para` : "Sinalizar para"}</MenuHeading>
          {FOLLOWUP_CHOICES.map(({ choice, label }) => {
            const due = dueFor(choice, today);
            const current = single?.followup && !single.followup.completed && single.flags.flagged ? single.followup.due : undefined;
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
          folders={destinations.map((f) => ({ id: f.id, name: f.name, icon: folderIcon(f.role) }))}
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

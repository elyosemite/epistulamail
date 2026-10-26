// Composição de DS: sidebar.html "Coluna de contas" (.account-rail, __list, __sep, __add) em .app-shell__rail, que na
// casca em blocos vai do topo ao rodapé sobre o fundo (espacamento.html) + menus.html (.menu de contexto)
import { useEffect, useRef, useState } from "react";
import { accountInitial, accountShortcut, avatarColorClass, railLabel, railTip, unreadBadge } from "../lib/accounts";
import type { Account } from "../lib/tauri";
import { Icon } from "../ui/Icon";
import { Menu, type MenuEntry } from "../ui/Menu";
import { AccountsBarItem } from "./AccountsBarItem";

interface AccountsBarProps {
  accounts: Account[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onEdit: (account: Account) => void;
  onMove: (account: Account, direction: -1 | 1) => void;
  onSignOut: (account: Account) => void;
  onReconnect: (account: Account) => void;
  /** Arquivo local (US15): remover do Epistula. */
  onRemove: (account: Account) => void;
}

/** Coluna com as contas do usuário, do topo ao rodapé da janela. */
export function AccountsBar({ accounts, activeId, onSelect, onAdd, onEdit, onMove, onSignOut, onReconnect, onRemove }: AccountsBarProps) {
  const [menu, setMenu] = useState<{ account: Account; anchor: { x: number; y: number } } | null>(null);
  const items = useRef(new Map<string, HTMLButtonElement>());

  // Ctrl+1 a Ctrl+9 trocam de conta de qualquer lugar do app.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const index = accountShortcut(e);
      if (index === null || !accounts[index]) return;
      e.preventDefault();
      onSelect(accounts[index].id);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [accounts, onSelect]);

  const focusAt = (i: number) => {
    const target = accounts[Math.max(0, Math.min(accounts.length - 1, i))];
    if (target) items.current.get(target.id)?.focus();
  };

  const entries = (a: Account, index: number): MenuEntry[] => {
    const access: MenuEntry =
      a.status === "local"
        ? { label: "Remover do Epistula…", icon: "trash", onSelect: () => onRemove(a) }
        : a.status === "active"
        ? { label: "Sair desta conta", icon: "log-out", onSelect: () => onSignOut(a) }
        : { label: a.status === "signed_out" ? "Entrar de novo" : "Reconectar", icon: "log-in", onSelect: () => onReconnect(a) };
    return [
      { label: "Renomear", icon: "pencil", onSelect: () => onEdit(a) },
      ...(a.status === "local" ? [] : [{ label: "Mudar cor", icon: "droplet", onSelect: () => onEdit(a) } as MenuEntry]),
      { label: "Mover para cima", icon: "arrow-up", onSelect: () => onMove(a, -1), disabled: index === 0 },
      { label: "Mover para baixo", icon: "arrow-down", onSelect: () => onMove(a, 1), disabled: index === accounts.length - 1 },
      "separator",
      access,
    ];
  };

  return (
    <>
      <nav className="account-rail app-shell__rail" aria-label="Contas">
        <div className="account-rail__list" role="list">
        {accounts.map((a, i) => (
          <AccountsBarItem
            key={a.id}
            ref={(el) => { if (el) items.current.set(a.id, el); else items.current.delete(a.id); }}
            name={a.displayName}
            initial={accountInitial(a.displayName)}
            colorClass={avatarColorClass(a.color)}
            photo={a.photo}
            archive={a.status === "local"}
            badge={unreadBadge(a.unread)}
            state={a.status === "signed_out" ? "signed-out" : a.status === "needs_reauth" ? "alert" : "normal"}
            current={a.id === activeId}
            label={railLabel(a)}
            tip={railTip(a, i)}
            onSelect={() => onSelect(a.id)}
            onMenu={(anchor) => setMenu({ account: a, anchor })}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") { e.preventDefault(); focusAt(i + 1); }
              else if (e.key === "ArrowUp") { e.preventDefault(); focusAt(i - 1); }
            }}
          />
        ))}
        </div>
        <div className="account-rail__sep" role="separator" />
        <button className="account-rail__add" type="button" aria-label="Adicionar conta ou abrir arquivo" data-tip="Adicionar conta ou abrir arquivo" onClick={onAdd}>
          <Icon name="plus" />
        </button>
      </nav>
      {menu && (
        <Menu
          label={`Conta ${menu.account.displayName}`}
          heading={menu.account.displayName}
          anchor={menu.anchor}
          entries={entries(menu.account, accounts.findIndex((a) => a.id === menu.account.id))}
          onClose={() => setMenu(null)}
        />
      )}
    </>
  );
}

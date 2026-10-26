// DS: menus.html "Menu da conta" · .account-trigger (foto na extrema direita do header) e .menu.account-menu com
//     __head, __name, __email, __info, __line (--warning), __pref, __label; .menu__sep; .menu-item; .segmented--fill (L40–L42);
//     "Bloquear o Epistula" acima de "Sair" com a senha ligada (header.html, US10)
import { useCallback, useEffect, useRef, useState } from "react";
import { accountInitial, avatarColorClass } from "../lib/accounts";
import { savedLabel, syncedLabel, unreadLabel } from "../lib/accountMenu";
import { DENSITY_CHOICES, THEME_CHOICES } from "../lib/appearance";
import { api, type Account, type AccountOverview, type Appearance } from "../lib/tauri";
import { AccountAvatar } from "../ui/AccountAvatar";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { MenuItem, MenuPanel, MenuSeparator, type Anchor } from "../ui/Menu";
import { Segmented } from "../ui/Segmented";

interface AccountMenuProps {
  account: Account;
  appearance: Appearance;
  /** A última sincronização falhou por falta de conexão. */
  offline: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAppearance: (change: Partial<Appearance>) => void;
  onSettings: () => void;
  onSignOut: () => void;
  /** Só com a senha do Epistula ligada (US10, CA3). */
  onLock?: () => void;
}

/** Foto da conta ativa e o menu da conta: quem você é, preferências rápidas e sair (US7, CA6). */
export function AccountMenu({ account, appearance, offline, open, onOpenChange, onAppearance, onSettings, onSignOut, onLock }: AccountMenuProps) {
  const trigger = useRef<HTMLButtonElement>(null);
  const closing = useRef(false);
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  const [overview, setOverview] = useState<AccountOverview | null>(null);
  const [syncing, setSyncing] = useState(false);

  const load = useCallback(() => {
    api.accountOverview(account.id).then(setOverview).catch(() => setOverview(null));
  }, [account.id]);

  // O menu abre alinhado pela direita da foto (o MenuPanel o mantém dentro da janela).
  useEffect(() => {
    if (!open) return setAnchor(null);
    const r = trigger.current?.getBoundingClientRect();
    setAnchor(r ? { x: r.right, y: r.bottom + 4 } : { x: innerWidth, y: 56 });
    load();
  }, [open, load]);

  const syncNow = async () => {
    setSyncing(true);
    await api.syncAccount(account.id).catch(() => undefined);
    setSyncing(false);
    load();
  };

  const active = account.status === "active";
  return (
    <>
      <button
        ref={trigger}
        type="button"
        className="account-trigger"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`Conta: ${account.displayName}`}
        data-tip="Conta (Alt+A)"
        // Clicar na foto com o menu aberto fecha (o clique fora do menu já o fechou no pointerdown).
        onPointerDown={() => { closing.current = open; }}
        onClick={() => {
          if (closing.current) closing.current = false;
          else onOpenChange(!open);
        }}
      >
        <AccountAvatar initial={accountInitial(account.displayName)} colorClass={avatarColorClass(account.color)} photo={account.photo} size="sm" />
      </button>
      {open && anchor && (
        <MenuPanel label="Conta" anchor={anchor} className="account-menu" onClose={() => onOpenChange(false)}>
          <div className="account-menu__head">
            <AccountAvatar initial={accountInitial(account.displayName)} colorClass={avatarColorClass(account.color)} photo={account.photo} size="lg" />
            <div style={{ minWidth: 0 }}>
              <div className="account-menu__name">{account.displayName}</div>
              <div className="account-menu__email">{account.email}</div>
            </div>
          </div>
          <div className="account-menu__info">
            {offline && active ? (
              <div className="account-menu__line account-menu__line--warning"><Icon name="warning" size="sm" /><span>Sem conexão</span></div>
            ) : (
              <div className="account-menu__line">
                <Icon name="refresh" size="sm" />
                <span>{syncing ? "Sincronizando…" : syncedLabel(overview?.lastSyncAt ?? null)}</span>
                {active && <Button appearance="subtle" compact disabled={syncing} onClick={() => void syncNow()}>Sincronizar agora</Button>}
              </div>
            )}
            {overview && (
              <>
                <div className="account-menu__line"><Icon name="mail" size="sm" /><span>{unreadLabel(overview.unread)}</span></div>
                <div className="account-menu__line"><Icon name="hard-drive" size="sm" /><span>{savedLabel(overview.saved, overview.localBytes)}</span></div>
              </>
            )}
          </div>
          <MenuSeparator />
          <MenuItem label="Configurações" icon="settings" kbd="Ctrl+," onSelect={() => { onOpenChange(false); onSettings(); }} />
          <div className="account-menu__pref">
            <span className="account-menu__label" id="account-menu-theme">Tema</span>
            <Segmented label="Tema" labelledBy="account-menu-theme" fill options={THEME_CHOICES} value={appearance.theme} onChange={(theme) => onAppearance({ theme })} />
          </div>
          <div className="account-menu__pref">
            <span className="account-menu__label" id="account-menu-density">Densidade da lista</span>
            <Segmented label="Densidade da lista" labelledBy="account-menu-density" fill options={DENSITY_CHOICES} value={appearance.density} onChange={(density) => onAppearance({ density })} />
          </div>
          <MenuSeparator />
          {onLock && <MenuItem label="Bloquear o Epistula" icon="lock" kbd="Ctrl+Shift+L" onSelect={() => { onOpenChange(false); onLock(); }} />}
          <MenuItem label={`Sair de ${account.displayName}`} icon="log-out" disabled={account.status === "signed_out"} onSelect={() => { onOpenChange(false); onSignOut(); }} />
        </MenuPanel>
      )}
    </>
  );
}

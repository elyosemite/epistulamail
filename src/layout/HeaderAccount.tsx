// DS: header.html "Conta conectada" · .appbar__account, .appbar__account-name; identidade.html · .avatar--sm com foto
import { accountInitial, avatarColorClass } from "../lib/accounts";
import type { Account } from "../lib/tauri";
import { AccountAvatar } from "../ui/AccountAvatar";

/** Conta ativa, à direita do header. */
export function HeaderAccount({ account }: { account: Account }) {
  return (
    <div className="appbar__account" role="status">
      <AccountAvatar initial={accountInitial(account.displayName)} colorClass={avatarColorClass(account.color)} photo={account.photo} size="sm" />
      <span className="appbar__account-text">
        <span className="appbar__account-name">{account.displayName}</span> · {account.email}
      </span>
    </div>
  );
}

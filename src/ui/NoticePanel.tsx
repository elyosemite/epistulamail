// DS: header.html "Sino de notificações" (US8, L52) · .bell com .badge.badge--primary.bell__count; .menu.notice-panel
//     (__head, __title, __list, __foot), .notice (.is-unread, __body, __title, __text, __time, __icon --danger/--warning)
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { Icon } from "./Icon";

interface BellButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  /** "2", "9+" ou nada. */
  count: string | null;
  /** Não perturbe ligado: sino riscado. */
  quiet: boolean;
}

export const BellButton = forwardRef<HTMLButtonElement, BellButtonProps>(function BellButton({ count, quiet, className, type = "button", ...rest }, ref) {
  return (
    <button ref={ref} type={type} className={["btn btn--subtle btn--icon bell", className].filter(Boolean).join(" ")} {...rest}>
      <Icon name={quiet ? "bell-off" : "bell"} />
      {count && <span className="badge badge--primary bell__count" aria-hidden="true">{count}</span>}
    </button>
  );
});

export function NoticePanelHead({ title, action }: { title: string; action?: ReactNode }) {
  return (
    <div className="notice-panel__head">
      <h3 className="notice-panel__title">{title}</h3>
      {action}
    </div>
  );
}

export const NoticeList = ({ children }: { children: ReactNode }) => <div className="notice-panel__list" role="list">{children}</div>;

export const NoticePanelFoot = ({ children }: { children: ReactNode }) => <div className="notice-panel__foot">{children}</div>;

/** Círculo com o ícone do tipo de aviso. */
export function NoticeIcon({ icon, tone }: { icon: string; tone: "neutral" | "danger" | "warning" }) {
  return (
    <span className={["notice__icon", tone !== "neutral" && `notice__icon--${tone}`].filter(Boolean).join(" ")} aria-hidden="true">
      <Icon name={icon} size="sm" />
    </span>
  );
}

interface NoticeRowProps {
  media: ReactNode;
  title: string;
  text: string;
  time: string;
  unread: boolean;
  onOpen?: () => void;
}

export function NoticeRow({ media, title, text, time, unread, onOpen }: NoticeRowProps) {
  return (
    <button
      type="button"
      className={["notice", unread && "is-unread"].filter(Boolean).join(" ")}
      role="listitem"
      aria-label={`${unread ? "Não vista, " : ""}${title}${text ? `, ${text}` : ""}, ${time}`}
      onClick={onOpen}
    >
      {media}
      <span className="notice__body">
        <span className="notice__title">{title}</span>
        {text && <span className="notice__text">{text}</span>}
      </span>
      <span className="notice__time">{time}</span>
    </button>
  );
}

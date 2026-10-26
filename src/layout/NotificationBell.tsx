// DS: header.html "Sino de notificações" (US8, CA7 e D12) · .bell no fim do header, entre os botões das colunas e a foto da
//     conta, com o painel .menu.notice-panel; vazios.html (.empty "Nada novo por aqui"); campos.html (.toggle do Não perturbe)
import { useCallback, useEffect, useRef, useState } from "react";
import { bellCount, bellLabel, noticeIcon, noticeText, noticeTime } from "../lib/notices";
import { api, onBackendEvent, type Notice, type NoticeList as List, type NoticeTarget } from "../lib/tauri";
import { Avatar } from "../ui/Avatar";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { MenuPanel, type Anchor } from "../ui/Menu";
import { BellButton, NoticeIcon, NoticeList, NoticePanelFoot, NoticePanelHead, NoticeRow } from "../ui/NoticePanel";

interface NotificationBellProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Clicar numa notificação: a conta e a mensagem, como na notificação do Windows. */
  onOpenTarget: (target: NoticeTarget) => void;
  /** Configurações → Notificações. */
  onSettings: () => void;
}

const EMPTY: List = { items: [], unseen: 0, quiet: false, snoozed: false };

/** O sino com as notificações dos últimos 7 dias. Abrir o painel marca todas como vistas. */
export function NotificationBell({ open, onOpenChange, onOpenTarget, onSettings }: NotificationBellProps) {
  const trigger = useRef<HTMLButtonElement>(null);
  const closing = useRef(false);
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  const [list, setList] = useState<List>(EMPTY);
  const openRef = useRef(open);
  openRef.current = open;

  const load = useCallback(() => {
    api.listNotices().then((next) => {
      // Com o painel aberto, as que chegarem já contam como vistas; o ponto azul fica até fechar.
      if (openRef.current && next.unseen > 0) {
        void api.markNoticesSeen().catch(() => undefined);
        setList({ ...next, unseen: 0 });
      } else {
        setList(next);
      }
    }).catch(() => undefined);
  }, []);

  useEffect(() => {
    load();
    return onBackendEvent("notices-updated", load);
  }, [load]);

  useEffect(() => {
    if (!open) {
      setAnchor(null);
      // Fechado: o ponto azul some das que já foram vistas.
      setList((l) => (l.items.some((n) => !n.seen) && l.unseen === 0 ? { ...l, items: l.items.map((n) => ({ ...n, seen: true })) } : l));
      return;
    }
    const r = trigger.current?.getBoundingClientRect();
    setAnchor(r ? { x: r.right, y: r.bottom + 4 } : { x: innerWidth, y: 56 });
    setList((l) => ({ ...l, unseen: 0 }));
    void api.markNoticesSeen().catch(() => undefined);
  }, [open]);

  const markAll = () => {
    void api.markNoticesSeen().catch(() => undefined);
    setList((l) => ({ ...l, unseen: 0, items: l.items.map((n) => ({ ...n, seen: true })) }));
  };

  const snooze = (on: boolean) => {
    setList((l) => ({ ...l, snoozed: on, quiet: on || l.quiet }));
    api.setSnooze(on).then(load).catch(load);
  };

  const pick = (notice: Notice) => {
    onOpenChange(false);
    if (notice.target) onOpenTarget(notice.target);
  };

  const media = (n: Notice) => {
    if (n.sender && (n.kind === "mail" || n.kind === "reply")) return <Avatar name={n.sender} />;
    const { icon, tone } = noticeIcon(n.kind);
    return <NoticeIcon icon={icon} tone={tone} />;
  };

  return (
    <>
      <BellButton
        ref={trigger}
        count={bellCount(list.unseen)}
        quiet={list.quiet}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={bellLabel(list.unseen, list.quiet)}
        data-tip={list.quiet ? "Notificações (Não perturbe) (Alt+N)" : "Notificações (Alt+N)"}
        // Clicar no sino com o painel aberto fecha (o clique fora do painel já o fechou no pointerdown).
        onPointerDown={() => { closing.current = open; }}
        onClick={() => {
          if (closing.current) closing.current = false;
          else onOpenChange(!open);
        }}
      />
      {open && anchor && (
        <MenuPanel label="Notificações" role="dialog" anchor={anchor} className="notice-panel" onClose={() => onOpenChange(false)}>
          <NoticePanelHead
            title="Notificações"
            action={list.items.some((n) => !n.seen) ? <Button appearance="subtle" compact onClick={markAll}>Marcar todas como vistas</Button> : undefined}
          />
          {list.items.length === 0 ? (
            <EmptyState title="Nada novo por aqui" description="Os avisos dos últimos 7 dias aparecem aqui." />
          ) : (
            <NoticeList>
              {list.items.map((n) => (
                <NoticeRow
                  key={n.id}
                  media={media(n)}
                  title={n.title}
                  text={noticeText(n.line1, n.line2)}
                  time={noticeTime(n.createdAt)}
                  unread={!n.seen}
                  onOpen={() => pick(n)}
                />
              ))}
            </NoticeList>
          )}
          <NoticePanelFoot>
            <input type="checkbox" className="toggle" id="notice-snooze" checked={list.snoozed} onChange={(e) => snooze(e.target.checked)} />
            <label htmlFor="notice-snooze">Não perturbe por 1 hora</label>
            <Button appearance="link" compact onClick={() => { onOpenChange(false); onSettings(); }}>Configurações</Button>
          </NoticePanelFoot>
        </MenuPanel>
      )}
    </>
  );
}

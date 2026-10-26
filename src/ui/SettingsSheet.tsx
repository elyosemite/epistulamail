// DS: modais.html "Configurações" · .app-shell__sheet, .settings-sheet (__head), .settings, .settings__nav (tablist vertical
//     de .nav-item), .settings__panel, .settings__title, .settings__group, .settings__group-title, .setting (__label, __desc),
//     .setting-list (__head, .is-selecting, __bulk, __row, .is-checked, .is-disabled, __name, __title, __sub),
//     .setting-list__row--toggle (US8, L49: linha com um toggle no fim)
// Cobre as duas colunas, sem cobrir a coluna de contas nem o header (D2). Esc fecha; o foco volta para onde estava.
import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from "react";
import { IconButton } from "./Button";
import { Icon } from "./Icon";

export interface SettingsSection {
  id: string;
  label: string;
  icon: string;
}

interface SettingsSheetProps {
  sections: readonly SettingsSection[];
  current: string;
  onSection: (id: string) => void;
  onClose: () => void;
  children: ReactNode;
}

export function SettingsSheet({ sections, current, onSection, onClose, children }: SettingsSheetProps) {
  const ref = useRef<HTMLDivElement>(null);
  const tabs = useRef(new Map<string, HTMLButtonElement>());
  const titleId = useId();
  const panelId = useId();

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    tabs.current.get(current)?.focus();
    return () => previous?.focus?.();
    // Só ao abrir: trocar de seção não devolve o foco.
  }, []);

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  const onTabKey = (e: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const next = sections[(index + step + sections.length) % sections.length];
    onSection(next.id);
    tabs.current.get(next.id)?.focus();
  };

  return (
    <div className="app-shell__sheet" onKeyDown={onKeyDown}>
      <div ref={ref} className="settings-sheet" role="dialog" aria-labelledby={titleId}>
        <div className="settings-sheet__head">
          <Icon name="settings" />
          <h2 className="modal__title" id={titleId}>Configurações</h2>
          <IconButton icon="x" label="Fechar" data-tip="Fechar (Esc)" onClick={onClose} />
        </div>
        <div className="settings">
          <div className="settings__nav" role="tablist" aria-orientation="vertical" aria-label="Seções">
            {sections.map((s, i) => (
              <button
                key={s.id}
                ref={(el) => { if (el) tabs.current.set(s.id, el); else tabs.current.delete(s.id); }}
                type="button"
                role="tab"
                className="nav-item"
                aria-selected={s.id === current}
                aria-controls={panelId}
                tabIndex={s.id === current ? 0 : -1}
                onClick={() => onSection(s.id)}
                onKeyDown={(e) => onTabKey(e, i)}
              >
                <Icon name={s.icon} />
                <span className="nav-item__text">{s.label}</span>
              </button>
            ))}
          </div>
          <div className="settings__panel" role="tabpanel" id={panelId} aria-label={sections.find((s) => s.id === current)?.label}>
            {children}
          </div>
        </div>
      </div>
    </div>
  );
}

export const SettingsTitle = ({ children }: { children: ReactNode }) => <h3 className="settings__title">{children}</h3>;

export function SettingsGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="settings__group">
      {title && <h4 className="settings__group-title">{title}</h4>}
      {children}
    </div>
  );
}

/** Ajuste: rótulo e descrição à esquerda, controle à direita. `labelId` liga o rótulo ao controle. */
export function Setting({ labelId, label, description, children }: { labelId: string; label: string; description?: string; children: ReactNode }) {
  return (
    <div className="setting">
      <div>
        <div className="setting__label" id={labelId}>{label}</div>
        {description && <p className="setting__desc">{description}</p>}
      </div>
      {children}
    </div>
  );
}

export const SettingNote = ({ children }: { children: ReactNode }) => <p className="setting__desc">{children}</p>;

/** Lista com uma linha por item e um cabeçalho que vira barra de ação em lote com seleção. */
export function SettingList({ head, children }: { head?: ReactNode; children: ReactNode }) {
  return (
    <div className="setting-list">
      {head}
      {children}
    </div>
  );
}

interface SettingListHeadProps {
  selecting: boolean;
  master: "none" | "some" | "all";
  masterLabel: string;
  disabled?: boolean;
  onMaster: () => void;
  children: ReactNode;
  bulk?: ReactNode;
}

export function SettingListHead({ selecting, master, masterLabel, disabled, onMaster, children, bulk }: SettingListHeadProps) {
  return (
    <div className={selecting ? "setting-list__head is-selecting" : "setting-list__head"}>
      <input
        type="checkbox"
        className="checkbox"
        aria-label={masterLabel}
        checked={master === "all"}
        ref={(el) => { if (el) el.indeterminate = master === "some"; }}
        disabled={disabled}
        onChange={onMaster}
      />
      <span aria-live="polite">{children}</span>
      {bulk && <div className="setting-list__bulk">{bulk}</div>}
    </div>
  );
}

interface SettingListRowProps {
  checked: boolean;
  disabled?: boolean;
  checkLabel: string;
  onCheck: () => void;
  media: ReactNode;
  title: string;
  subtitle: string;
  control: ReactNode;
}

export function SettingListRow({ checked, disabled, checkLabel, onCheck, media, title, subtitle, control }: SettingListRowProps) {
  const cls = ["setting-list__row", checked && "is-checked", disabled && "is-disabled"].filter(Boolean).join(" ");
  return (
    <div className={cls}>
      <input type="checkbox" className="checkbox" aria-label={checkLabel} checked={checked} disabled={disabled} onChange={onCheck} />
      {media}
      <div className="setting-list__name">
        <span className="setting-list__title">{title}</span>
        <span className="setting-list__sub">{subtitle}</span>
      </div>
      {control}
    </div>
  );
}

/** Linha com avatar, nome e um controle no fim, sem seleção (notificações por conta, US8; espaço guardado, US10). */
export function SettingToggleRow({ media, title, subtitle, control, disabled }: { media: ReactNode; title: string; subtitle: string; control: ReactNode; disabled?: boolean }) {
  return (
    <div className={["setting-list__row setting-list__row--toggle", disabled && "is-disabled"].filter(Boolean).join(" ")}>
      {media}
      <div className="setting-list__name">
        <span className="setting-list__title">{title}</span>
        <span className="setting-list__sub">{subtitle}</span>
      </div>
      {control}
    </div>
  );
}

/** Folha de um gerenciador aberto a partir das Configurações (criptografia.html, L66): "Voltar", título e fechar, sem a navegação. */
export function ManagerSheet({ title, onBack, onClose, children }: { title: string; onBack: () => void; onClose: () => void; children: ReactNode }) {
  const titleId = useId();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => ref.current?.querySelector<HTMLElement>(".settings-sheet__back")?.focus(), []);
  return (
    <div
      className="app-shell__sheet"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          onBack();
        }
      }}
    >
      <div ref={ref} className="settings-sheet" role="dialog" aria-labelledby={titleId}>
        <div className="settings-sheet__head">
          <IconButton icon="chevron-left" label="Voltar às configurações" className="settings-sheet__back" onClick={onBack} />
          <h2 className="modal__title" id={titleId}>{title}</h2>
          <IconButton icon="x" label="Fechar" data-tip="Fechar" onClick={onClose} />
        </div>
        {children}
      </div>
    </div>
  );
}

// DS: regras.html "Configurações → Regras" · .rule-list com .rule-row (__grip, __body, __name, __stats, __warn; .is-off,
// .is-dragging) (L59), .toggle, .menu da regra; a frase em RuleSentence (L58)
import { useRef, useState, type DragEvent, type KeyboardEvent } from "react";
import { emailsLabel, isCleanup, sentence } from "../../lib/rules";
import type { Folder, Rule } from "../../lib/tauri";
import { Icon } from "../../ui/Icon";
import { MenuItem, MenuPanel, MenuSeparator, anchorBelow, type Anchor } from "../../ui/Menu";
import { RuleSentence } from "../../ui/RuleSentence";

interface RuleListProps {
  rules: Rule[];
  folders: Folder[];
  onToggle: (rule: Rule, enabled: boolean) => void;
  onEdit: (rule: Rule) => void;
  onRun: (rule: Rule) => void;
  onDuplicate: (rule: Rule) => void;
  onDelete: (rule: Rule) => void;
  /** Nova ordem, com os ids na ordem em que rodam. */
  onReorder: (ids: number[]) => void;
}

const PROBLEMS: Record<string, string> = {
  folder_missing: "Desligada: a pasta de destino não existe mais. Edite para escolher outra.",
};

/** Lista das regras da conta, na ordem em que rodam (CA1). Arrastar pela alça ou Alt+↑/↓ muda a ordem. */
export function RuleList({ rules, folders, onToggle, onEdit, onRun, onDuplicate, onDelete, onReorder }: RuleListProps) {
  const [menu, setMenu] = useState<{ rule: Rule; anchor: Anchor } | null>(null);
  const [dragging, setDragging] = useState<number | null>(null);
  const rows = useRef(new Map<number, HTMLDivElement>());

  const move = (id: number, to: number) => {
    const ids = rules.map((r) => r.id).filter((x) => x !== id);
    ids.splice(Math.max(0, Math.min(to, ids.length)), 0, id);
    onReorder(ids);
  };
  const step = (rule: Rule, delta: -1 | 1) => {
    const at = rules.findIndex((r) => r.id === rule.id);
    if (at + delta < 0 || at + delta >= rules.length) return;
    move(rule.id, at + delta);
    requestAnimationFrame(() => rows.current.get(rule.id)?.focus());
  };

  const onKey = (rule: Rule) => (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      step(rule, e.key === "ArrowUp" ? -1 : 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      onEdit(rule);
    }
  };

  const onDrop = (target: Rule) => (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    if (dragging === null || dragging === target.id) return;
    move(dragging, rules.findIndex((r) => r.id === target.id));
    setDragging(null);
  };

  return (
    <>
      <div className="rule-list" role="list" aria-label="Regras, na ordem em que rodam">
        {rules.map((rule) => {
          const s = sentence(rule.spec, folders);
          const cls = ["rule-row", !rule.enabled && "is-off", dragging === rule.id && "is-dragging"].filter(Boolean).join(" ");
          return (
            <div
              key={rule.id}
              ref={(el) => { if (el) rows.current.set(rule.id, el); else rows.current.delete(rule.id); }}
              className={cls}
              role="listitem"
              tabIndex={0}
              aria-label={`${rule.name}. Alt e setas mudam a ordem; Enter edita.`}
              draggable
              onDragStart={(e) => { setDragging(rule.id); e.dataTransfer.effectAllowed = "move"; }}
              onDragEnd={() => setDragging(null)}
              onDragOver={(e) => e.preventDefault()}
              onDrop={onDrop(rule)}
              onKeyDown={onKey(rule)}
              onDoubleClick={() => onEdit(rule)}
            >
              <span className="rule-row__grip" aria-hidden="true"><Icon name="grip" /></span>
              <input type="checkbox" className="toggle" checked={rule.enabled} aria-label={`Ligar a regra ${rule.name}`} onChange={(e) => onToggle(rule, e.target.checked)} />
              <div className="rule-row__body">
                <span className="rule-row__name">{rule.name}</span>
                <RuleSentence conditions={s.conditions} exceptions={s.exceptions} actions={s.actions} joiner={s.joiner} stop={rule.spec.stop} />
                {rule.problem && (
                  <span className="rule-row__warn"><Icon name="warning" size="sm" />{PROBLEMS[rule.problem] ?? "Desligada por um erro. Edite para conferir."}</span>
                )}
              </div>
              <span className="rule-row__stats">
                {isCleanup(rule.spec) ? <>Limpeza diária<br /></> : null}
                {rule.hits30d > 0 ? <>{emailsLabel(rule.hits30d)}<br />em 30 dias</> : "—"}
              </span>
              <button
                type="button"
                className="btn btn--subtle btn--icon btn--compact"
                aria-label={`Mais ações da regra ${rule.name}`}
                aria-haspopup="menu"
                onClick={(e) => setMenu({ rule, anchor: anchorBelow(e.currentTarget) })}
              >
                <Icon name="more" />
              </button>
            </div>
          );
        })}
      </div>
      {menu && (
        <MenuPanel label={`Regra ${menu.rule.name}`} anchor={menu.anchor} onClose={() => setMenu(null)}>
          <MenuItem label="Editar" icon="pencil" onSelect={() => { setMenu(null); onEdit(menu.rule); }} />
          <MenuItem label="Rodar agora…" icon="play" onSelect={() => { setMenu(null); onRun(menu.rule); }} />
          <MenuItem label="Duplicar" icon="copy" onSelect={() => { setMenu(null); onDuplicate(menu.rule); }} />
          <MenuItem label="Subir" icon="arrow-up" kbd="Alt+↑" disabled={rules[0]?.id === menu.rule.id} onSelect={() => { setMenu(null); step(menu.rule, -1); }} />
          <MenuItem label="Descer" icon="arrow-down" kbd="Alt+↓" disabled={rules[rules.length - 1]?.id === menu.rule.id} onSelect={() => { setMenu(null); step(menu.rule, 1); }} />
          <MenuSeparator />
          <MenuItem label="Excluir regra" icon="trash" onSelect={() => { setMenu(null); onDelete(menu.rule); }} />
        </MenuPanel>
      )}
    </>
  );
}

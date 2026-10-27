// DS: header.html · .appbar__search com .input.input--search; busca.html (US16) · .chip-input com .chip de pessoa
//     (.chip__op De/Para/Cc, .avatar, .is-selected no Backspace) e .chip--filter (L73); sugestões num .menu.menu--wide
//     (.menu__heading, .menu-item com .menu-item__meta e kbd do operador); menu de período com .menu__form
import { forwardRef, useEffect, useImperativeHandle, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import {
  addChip,
  chipText,
  fromQuery,
  parseOperators,
  periodLabel,
  PERIOD_PRESETS,
  presetChip,
  ROLE_LABEL,
  typingAt,
  withoutTrailingOp,
  type PersonRole,
  type Search,
  type SearchChip,
} from "../lib/search";
import { folderIcon } from "../lib/folders";
import { api, type Folder, type RecentSearch, type SearchContact } from "../lib/tauri";
import { Avatar } from "../ui/Avatar";
import { IconButton } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { anchorBelow, MenuHeading, MenuItem, MenuPanel, MenuSeparator, type Anchor } from "../ui/Menu";

interface HeaderSearchProps {
  accountId: string;
  search: Search;
  placeholder: string;
  /** Com as configurações abertas, a busca não responde (D2). */
  inert?: boolean;
  onChange: (search: Search) => void;
  onEscape: () => void;
}

/** Uma sugestão da lista: o texto, o ícone ou avatar e o que acontece ao escolher. */
interface Option {
  key: string;
  label: string;
  meta?: string;
  kbd?: string;
  icon?: string;
  avatar?: { name: string; seed: string };
  pick: () => void;
}
type Group = { heading: string; options: Option[] };

const CONTACTS_DELAY_MS = 150;
const ROLE_SEARCH: Record<PersonRole, string> = { from: "no remetente", to: "nos destinatários", cc: "em Cc", with: "em De, Para ou Cc" };
const FILTER_ICON = { attachment: "clip", unread: "mail", starred: "star", period: "clock", folder: "folder" } as const;

export const HeaderSearch = forwardRef<HTMLInputElement, HeaderSearchProps>(function HeaderSearch(
  { accountId, search, placeholder, inert, onChange, onEscape },
  ref,
) {
  const input = useRef<HTMLInputElement>(null);
  useImperativeHandle(ref, () => input.current!, []);
  const box = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const [picked, setPicked] = useState<number | null>(null);
  const [contacts, setContacts] = useState<SearchContact[]>([]);
  const [recent, setRecent] = useState<RecentSearch[]>([]);
  const [folders, setFolders] = useState<Folder[] | null>(null);
  const [period, setPeriod] = useState<Anchor | null>(null);
  const [anchor, setAnchor] = useState<{ x: number; y: number; width: number } | null>(null);
  const listId = "search-suggestions";

  const typing = typingAt(search.text);
  const personTerm = typing?.kind === "person" ? typing.term : null;

  // Pessoas para "de:", "para:", "cc:" e "com:", das que mais aparecem.
  useEffect(() => {
    if (personTerm === null || !open) return;
    const timer = setTimeout(() => {
      api.searchContacts(accountId, personTerm, search.allAccounts).then(setContacts).catch(() => setContacts([]));
    }, CONTACTS_DELAY_MS);
    return () => clearTimeout(timer);
  }, [accountId, personTerm, search.allAccounts, open]);

  useEffect(() => setFolders(null), [accountId]);

  const show = () => {
    const r = box.current?.getBoundingClientRect();
    if (r) setAnchor({ x: r.left, y: r.bottom + 4, width: r.width });
    setOpen(true);
    api.recentSearches().then(setRecent).catch(() => setRecent([]));
    if (folders === null) api.listFolders(accountId).then(setFolders).catch(() => setFolders([]));
  };

  const update = (next: Partial<Search>) => onChange({ ...search, ...next });
  const pushChip = (chip: SearchChip, text = withoutTrailingOp(search.text)) => {
    update({ chips: addChip(search.chips, chip), text });
    setActive(-1);
    input.current?.focus();
  };
  const insertOp = (op: string) => {
    update({ text: `${search.text.trim()} ${op}`.trimStart() });
    setActive(0);
    input.current?.focus();
  };

  const groups = ((): Group[] => {
    if (typing?.kind === "person") {
      const role = typing.role;
      const people: Option[] = contacts.map((c) => ({
        key: `p-${c.address}`,
        label: c.name || c.address,
        meta: c.name ? `${c.address} · ${c.count}` : String(c.count),
        avatar: { name: c.name || c.address, seed: c.address },
        pick: () => pushChip({ kind: "person", role, name: c.name, address: c.address }),
      }));
      const free: Option[] = typing.term
        ? [{ key: "free", label: `Buscar "${typing.term}" ${ROLE_SEARCH[role]}`, icon: "search", pick: () => pushChip({ kind: "person", role, name: typing.term, address: null }) }]
        : [];
      return [{ heading: "Pessoas", options: people }, { heading: "", options: free }];
    }
    if (typing?.kind === "folder") {
      const term = typing.term.toLowerCase();
      const matches = (folders ?? []).filter((f) => f.name.toLowerCase().includes(term));
      return [{ heading: "Pastas", options: matches.map((f) => ({ key: `f-${f.id}`, label: f.name, icon: folderIcon(f.role), pick: () => pushChip({ kind: "folder", id: f.id, name: f.name }) })) }];
    }
    if (search.text.trim()) return [];
    const recents: Option[] = recent.slice(0, 5).map((r, i) => ({
      key: `r-${i}`,
      label: r.label,
      icon: "history",
      pick: () => {
        const back = fromQuery(r.query, folders?.find((f) => f.id === r.query.folder)?.name);
        onChange({ ...back, chips: back.chips.filter((c) => c.kind !== "folder" || !back.allAccounts) });
        setOpen(false);
      },
    }));
    const has = (kind: SearchChip["kind"]) => search.chips.some((c) => c.kind === kind);
    const filters: Option[] = [
      { key: "op-person", label: "De, Para ou Cc…", icon: "user", kbd: "de:", pick: () => insertOp("de: ") },
      { key: "op-period", label: "Período…", icon: "clock", kbd: "depois:", pick: () => openPeriod() },
      ...(has("attachment") ? [] : [{ key: "op-attachment", label: "Com anexo", icon: "clip", kbd: "tem:anexo", pick: () => pushChip({ kind: "attachment" }) }]),
      ...(has("unread") ? [] : [{ key: "op-unread", label: "Não lidas", icon: "mail", kbd: "é:não-lida", pick: () => pushChip({ kind: "unread" }) }]),
      ...(has("starred") ? [] : [{ key: "op-starred", label: "Com estrela", icon: "star", kbd: "é:estrela", pick: () => pushChip({ kind: "starred" }) }]),
      ...(search.allAccounts ? [] : [{ key: "op-folder", label: "Numa pasta…", icon: "folder", kbd: "na:", pick: () => insertOp("na: ") }]),
    ];
    return [{ heading: "Recentes", options: recents }, { heading: "Filtros", options: filters }];
  })();

  const options = groups.flatMap((g) => g.options);
  const expanded = open && options.length > 0 && !inert;

  // Ao digitar um operador, a primeira sugestão já fica escolhida; fora disso, Enter busca o que foi digitado.
  useEffect(() => setActive(typing ? 0 : -1), [typing?.kind]);

  const openPeriod = () => {
    setOpen(false);
    if (box.current) setPeriod(anchorBelow(box.current));
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== "Backspace") setPicked(null);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (!expanded) return show();
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      // Percorre as sugestões e volta ao campo (-1) entre a última e a primeira.
      setActive((i) => {
        const next = i + step;
        return next >= options.length ? -1 : next < -1 ? options.length - 1 : next;
      });
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (expanded && active >= 0 && options[active]) return options[active].pick();
      const parsed = parseOperators(search.text);
      if (parsed.chips.length > 0) update({ chips: parsed.chips.reduce(addChip, search.chips), text: parsed.text });
      setOpen(false);
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (expanded) return setOpen(false);
      onEscape();
    } else if (e.key === "Backspace" && e.currentTarget.selectionStart === 0 && e.currentTarget.selectionEnd === 0 && search.chips.length > 0) {
      // Primeira vez marca o último chip; a segunda tira (como no "Para").
      e.preventDefault();
      const last = search.chips.length - 1;
      if (picked === last) {
        update({ chips: search.chips.slice(0, last) });
        setPicked(null);
      } else {
        setPicked(last);
      }
    }
  };

  const removeChip = (i: number) => {
    update({ chips: search.chips.filter((_, j) => j !== i) });
    setPicked(null);
    input.current?.focus();
  };

  const currentPeriod = search.chips.find((c): c is Extract<SearchChip, { kind: "period" }> => c.kind === "period") ?? null;
  const setRange = (after: string | null, before: string | null) => {
    const others = search.chips.filter((c) => c.kind !== "period");
    update({ chips: after || before ? [...others, { kind: "period", after, before, label: periodLabel(after, before) }] : others });
  };

  const hasAnything = search.chips.length > 0 || search.text.length > 0;
  let index = -1;

  return (
    <div className="appbar__search">
      <div ref={box} className="input input--search chip-input" inert={inert} onClick={(e) => e.target === e.currentTarget && input.current?.focus()}>
        <Icon name="search" />
        {search.chips.map((chip, i) => (
          <SearchChipView key={`${chip.kind}-${i}`} chip={chip} selected={picked === i} onEdit={chip.kind === "period" ? openPeriod : undefined} onRemove={() => removeChip(i)} />
        ))}
        <input
          ref={input}
          type="search"
          role="combobox"
          aria-expanded={expanded}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={expanded && active >= 0 ? `${listId}-${active}` : undefined}
          value={search.text}
          placeholder={search.chips.length > 0 ? "" : placeholder}
          aria-label="Buscar e-mails nas mensagens salvas"
          onFocus={show}
          onBlur={() => setOpen(false)}
          onChange={(e) => {
            update({ text: e.target.value });
            setPicked(null);
            if (!open) show();
          }}
          onKeyDown={onKeyDown}
        />
        {hasAnything ? <IconButton icon="x" label="Limpar busca" compact onClick={onEscape} /> : <kbd>/</kbd>}
      </div>
      {expanded && anchor && (
        <div
          className="menu menu--wide"
          id={listId}
          role="listbox"
          aria-label="Sugestões"
          style={{ position: "fixed", left: anchor.x, top: anchor.y, width: anchor.width, zIndex: "var(--z-layer)" }}
          // O foco fica no campo: clicar numa sugestão não pode tirá-lo de lá.
          onMouseDown={(e) => e.preventDefault()}
        >
          {groups.map((g, gi) => g.options.length === 0 ? null : (
            <div key={gi} role="presentation">
              {gi > 0 && groups.slice(0, gi).some((p) => p.options.length > 0) && <MenuSeparator />}
              {g.heading && <MenuHeading>{g.heading}</MenuHeading>}
              {g.options.map((o) => {
                index++;
                const i = index;
                return (
                  <div
                    key={o.key}
                    id={`${listId}-${i}`}
                    className={["menu-item", i === active && "is-selected"].filter(Boolean).join(" ")}
                    role="option"
                    aria-selected={i === active}
                    onMouseEnter={() => setActive(i)}
                    onClick={() => o.pick()}
                  >
                    {o.avatar ? <Avatar name={o.avatar.name} seed={o.avatar.seed} size="sm" /> : <Icon name={o.icon ?? "search"} />}
                    <span className="menu-item__text">{o.label}</span>
                    {o.meta && <span className="menu-item__meta">{o.meta}</span>}
                    {o.kbd && <kbd>{o.kbd}</kbd>}
                  </div>
                );
              })}
            </div>
          ))}
          {recent.length > 0 && !search.text.trim() && (
            <>
              <MenuSeparator />
              <div
                className="menu-item"
                role="option"
                aria-selected={false}
                onClick={() => void api.clearRecentSearches().then(() => setRecent([])).catch(() => undefined)}
              >
                <Icon name="trash" />
                <span className="menu-item__text">Limpar buscas recentes</span>
              </div>
            </>
          )}
        </div>
      )}
      {period && (
        <MenuPanel label="Período" anchor={period} wide onClose={() => setPeriod(null)}>
          <MenuHeading>Período</MenuHeading>
          {PERIOD_PRESETS.map((p) => (
            <MenuItem
              key={p.id}
              label={p.label}
              icon="clock"
              selected={currentPeriod?.label === p.label}
              onSelect={() => {
                pushChip(presetChip(p.id), search.text);
                setPeriod(null);
              }}
            />
          ))}
          <MenuItem
            label="Qualquer data"
            icon="clock"
            selected={!currentPeriod}
            onSelect={() => {
              setRange(null, null);
              setPeriod(null);
            }}
          />
          <MenuSeparator />
          <MenuHeading>Intervalo</MenuHeading>
          <div className="menu__form">
            <div className="input"><input type="date" aria-label="De" value={currentPeriod?.after ?? ""} onChange={(e) => setRange(e.target.value || null, currentPeriod?.before ?? null)} /></div>
            <span style={{ color: "var(--color-text-subtle)" }}>a</span>
            <div className="input"><input type="date" aria-label="Até" value={currentPeriod?.before ?? ""} onChange={(e) => setRange(currentPeriod?.after ?? null, e.target.value || null)} /></div>
          </div>
        </MenuPanel>
      )}
    </div>
  );
});

/** Um chip do campo: pessoa (com o papel e o avatar) ou filtro (L73, com o ícone). */
function SearchChipView({ chip, selected, onEdit, onRemove }: { chip: SearchChip; selected: boolean; onEdit?: () => void; onRemove: () => void }) {
  const text = chipText(chip);
  let body: ReactNode;
  if (chip.kind === "person") {
    const who = chip.name || chip.address || "";
    const role = ROLE_LABEL[chip.role];
    body = (
      <>
        {role && <span className="chip__op">{role}</span>}
        <Avatar name={who} seed={chip.address ?? who} size="xs" />
        <span className="chip__name">{who}</span>
      </>
    );
  } else {
    const name = chip.kind === "period" ? chip.label : chip.kind === "folder" ? `Em ${chip.name}` : text.charAt(0).toUpperCase() + text.slice(1);
    body = (
      <>
        <Icon name={FILTER_ICON[chip.kind]} />
        <span className="chip__name">{name}</span>
      </>
    );
  }
  return (
    <span className={["chip", chip.kind !== "person" && "chip--filter", selected && "is-selected"].filter(Boolean).join(" ")} title={chip.kind === "person" ? chip.address ?? undefined : undefined}>
      {onEdit ? <button type="button" className="chip__body" aria-label={`Mudar o período: ${text}`} onClick={onEdit}>{body}</button> : body}
      <button className="chip__remove" type="button" aria-label={`Tirar o filtro ${text}`} onClick={onRemove}>
        <Icon name="x" size="sm" />
      </button>
    </span>
  );
}


# AGENT.md

Guidance for AI coding agents (and humans) working in this repository. Read it before changing anything. When this file and the code disagree, the code wins: fix this file in the same change.

## What this project is

EpistulaMail is a desktop email client for Windows 10/11 (Tauri 2 + Rust + React). Mail lives on the user's computer in a fully encrypted SQLCipher database; the Rust process does all network, crypto and HTML cleaning, and the WebView only displays. Privacy and security are the product, not a feature.

| Path | What it is | Ships via |
|---|---|---|
| `src/` | React 19 + TypeScript interface (Vite) | the installer |
| `src-tauri/` | Rust backend: OAuth, IMAP, SMTP, sync, SQLCipher, OpenPGP, S/MIME, rules, search, import | the installer (`release.yml`) |
| `design-system/` | The design system: `tokens.css`, `components.css`, `docs.js` (icon sprite + doc shell) and one HTML page per component | consumed by app, site and docs |
| `site/` | Public landing page, epistulamail.com (static, no framework) | Vercel (`site.yml`) |
| `docs-site/` | User docs, docs.epistulamail.com (Astro Starlight, 5 languages) | Vercel (`docs.yml`) |
| `docs/` | Architecture and threat model (`arquitetura-e-ameacas.md`) | — |
| `userstory/` | One file per delivery (`00NN-*.md`): context, acceptance criteria (CA), interface, decisions (D) | — |
| `CHANGELOG.md` | Keep a Changelog, in Portuguese; the release notes come from it | — |

## Language

- **Portuguese (pt-BR)** is the working language: UI strings, error messages, code comments, user stories, the changelog, design-system pages and commit messages. Match it. Write new comments in Portuguese, like the surrounding code.
- `README.md` is English, `README.pt-BR.md` is its Portuguese twin: change both together.
- The docs site is pt-BR by default plus `en`, `es`, `fr`, `it` (see Docs below).
- This file is in English on purpose.
- User-facing text is plain, friendly and concrete. No jargon where a common word works ("arquivos anexados", not "attachments payload"). The product name is **EpistulaMail** (older code and docs still say "Epistula"; leave internal identifiers like `epistula.db` alone).

## Golden rules

1. **The design system comes first.** No new screen, component or visual change goes straight into `src/`, `site/` or `docs-site/`. It is drafted in `design-system/`, approved, then implemented. See Design system.
2. **Never commit secrets.** `.env`, `client_secret_*.json`, signing keys and tokens are gitignored and gitleaks scans the full history in CI. Do not print them, paste them into files, or send them anywhere.
3. **No user content in logs.** No subject, sender, address, body or account email in `tracing` calls, errors or panics. There is a test that fails if markers leak into logs or into the database file in plain text.
4. **Nothing third-party at runtime.** No CDN, remote fonts, analytics, cookies or trackers in the app, the site or the docs. Fonts come from `@fontsource` packages; icons from the DS sprite.
5. **Security-sensitive changes cite the threat model.** Touching credentials, storage, crypto, network, HTML rendering or link handling means reading `docs/arquitetura-e-ameacas.md`, saying which threat (by number) is affected, testing it, and updating the doc if the analysis changed.
6. **Keep the three versions in lockstep.** `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` must have the same version, and `CHANGELOG.md` must have a matching `## [X.Y.Z]` section, or the release fails.
7. **Commit or push only when asked.** Commits use Conventional Commits in Portuguese without accents in the subject, scoped by area when useful: `feat: docs, ...`, `feat: site, ...`, `feat: US16, ...`, `fix: ...`, `build: ...`, `release: ...`, `docs: ...`. Optional body with `-` bullets explaining what and why.

## Design system (`design-system/`)

The DS is based on the Atlassian Design System, adapted to a dense inbox. It targets **WCAG 2.2 AA**, light and dark themes. It is the single source of visual truth for the app, the site and the docs.

### Files and how they are consumed

- `tokens.css`: every color, type, space, radius, elevation, layout, motion, focus, opacity, icon-size and breakpoint value, per theme (light, dark, and high contrast via `prefers-contrast: more` / `data-contrast="more"`). **Always use the semantic token** (`var(--color-text-subtle)`), never a raw hex, px value or font name outside this file, and **never a `--palette-*` primitive** (only `tokens.css` uses those; `pnpm ds:check` fails otherwise).
- `components.css`: all component classes (BEM-like: `.btn`, `.btn--primary`, `.thread__subject`, state classes `.is-*`). Forced state classes (`.is-hover`, `.is-focus`, `.is-active`) exist only so the DS pages can show every state side by side.
- `fichas.js`: the data behind the "Ficha" that `docs.js` renders at the end of every component page (where it is used, do and don't, content and translation, tests, platform notes, related pages, history). A new component page gets a new entry with the same id. Fichas never name CSS classes.
- `CONTRIBUTING.md`, `DECISIONS.md`, `ROADMAP.md` (inside `design-system/`): the contribution flow and review gates, the log of lasting decisions, and the order of work. Read `CONTRIBUTING.md` before proposing a change and add an entry to `DECISIONS.md` for any decision with lasting effect.
- `docs.js`: the shared shell of the DS pages, the page registry `EPISTULA_PAGES` (with maturity status), and **the icon sprite**. There is no `icons.svg` (decision L2): the app (`vite.config.ts`, `virtual:ds-icons`), the site (`site/build.mjs`) and the docs (`docs-site/scripts/sync-ds.mjs`) all extract the sprite from `docs.js` with a regex. Do not change the `insertAdjacentHTML('afterbegin', "...")` shape without updating all three extractors.
- The app loads `tokens.css` and `components.css` directly (`index.html`). The site and the docs **copy** them at build time; never edit the copies (`site/dist/`, `docs-site/src/styles/ds-*.css`, `docs-site/public/ds/`, `docs-site/src/generated/`).
- The block of `components.css` between `/* @docs-site:start */` and `/* @docs-site:end */` is what the docs site gets inline (page glow, gradient title, brand icons, language menu, timeline). Keep that block self-contained and free of app class names that collide with Starlight.

### Layers and maturity (`governanca.html`)

- Every DS page sits in one layer in `EPISTULA_PAGES`: **Fundamentos** (values and rules), **Core** (generic components), **Padrões** (recurring compositions of Core), **Produto** (email concepts) or **Layouts e páginas**. Put a piece in the lowest layer that fits; nothing with email semantics goes to Core. A generic component needs three real uses before it reaches Core (foundations, accessibility and security are exceptions, recorded in the user story).
- Each registry entry is `[id, title, description, status]`. `status` is `draft`, `experimental`, `beta`, `stable`, `deprecated` or `removed`; `docs.js` shows it as a lozenge in the overview and next to the page `h1`. New pages start at `beta` or lower; nothing `stable` is removed without a deprecation note (reason, alternative, how to migrate).
- Tokens have three levels: primitive (`--palette-*`, raw value, only in `tokens.css`), semantic (meaning, what components use) and component (rare, a local custom property such as `--btn-bg` declared in `components.css`, only when no semantic token fits). Opacity, focus ring, icon size and breakpoints have their own tokens; use them instead of loose values. `@media` widths in `components.css` must match a `--breakpoint-*` token.
- Layout rules use logical properties (`margin-inline-start`, `inset-inline-end`, `text-align: start`) so RTL works. A physical `left`/`right` is allowed only when the line carries a `/* físico: ... */` comment saying why.
- Pattern pages (`padrao-*.html`) follow the pattern template: problem, context, composition, flow, variations, states, accessibility, content, anti-patterns. Component pages end with the ficha. Security states keep their own text, icon and shape (never one green indicator for different concepts); see `criptografia.html`.
- Review gates before approving a piece: layer, reuse, tokens and themes, API and accessibility, content and localization, documentation and tests. `governanca.html` is the reference, and `pnpm ds:check` (also run in CI) checks what can be checked: token references, dark-theme parity, palette use, breakpoints, the page registry, and that every `// DS: x.html` comment in `src/` points to an existing page. Run it after any change in `design-system/`; `pnpm ds:check --metrics` prints the health numbers.

### Pieces, numbering and traceability

- New DS pieces get a sequential **L number** (`L58`, `L73`, `L76`, `L80`...). Find the highest one in use (`grep -rn "L[0-9][0-9]" design-system`) and continue from it. Mention it in the DS page, in the user story and in the commit (`peças L77-L80`).
- In DS pages, prose (ledes, guidance, captions, headings) describes behaviour in product language and never names CSS classes. Class names go in one `<p class="classes">` line at the end of each section.
- DS pages reference the user story and acceptance criteria they serve (`US12 · L58–L63`, `CA7`).
- React components in `src/ui/` start with a comment pointing to their DS source, e.g. `// DS: botoes.html · .btn, .btn--primary, ...`. Keep it accurate when you change the component.

### Workflow for any visual change

1. **Draft.** Create `design-system/rascunho-<topic>.html` (e.g. `rascunho-us16.html`) using only tokens and existing components, showing every state in both themes. New CSS goes in `components.css` with tokens only.
2. **Approve.** Show it to the user. Do not implement in the app before approval.
3. **Incorporate.** Move the pieces into the proper component page (or create one: `<id>.html` with `<body data-page="<id>">`, registered in `EPISTULA_PAGES` in `docs.js`), with the guidance blocks (where to use, accessibility, keyboard). Delete the draft unless the user asks to keep it, and record that in the user story.
4. **Implement** in the app/site/docs using the same class names.

### Requirements every piece must meet

- **Principles** (`principios.html`), in priority order when two components disagree: density with breathing room; reading first; one gesture per action (every destructive action offers Undo); state always visible.
- **Contrast**: text 4.5:1; icons, field borders and focus ring 3:1, in both themes (`acessibilidade.html` measures them live).
- **Sizes**: text at least 12px (11px only for uppercase labels at weight 600+). Hit targets at least 24×24px; 32px default on touch, 44px for quick actions.
- **Focus**: 2px ring in `--color-border-focused`, 2px offset, `:focus-visible` only.
- **Not color alone**: state changes shape and weight too (unread = weight 600 + dot + date emphasis).
- **Motion**: honor `prefers-reduced-motion` (durations to 0, skeletons stop).
- **Screen readers**: proper roles and `aria-label`s; live regions (`aria-live="polite"`) for toasts and previews.
- **Keyboard**: every action reachable by keyboard; respect the shortcut table in `acessibilidade.html` and add new shortcuts there.
- Decorative gradients and glows (L77/L78) are for featured page headers only, never behind long text and never in the email app.

## App frontend (`src/`)

- React 19, TypeScript strict, Vite, Vitest. Layout in `src/layout/`, DS-backed primitives in `src/ui/`, feature screens in `src/features/`, pure logic in `src/lib/`.
- Use the DS classes through `src/ui/` components; do not add ad-hoc CSS or inline styles that bypass tokens.
- Talk to the backend only through `src/lib/tauri.ts`. Backend errors arrive as `{ kind, message }` where `message` is already user-facing Portuguese: show it, do not rebuild it.
- The WebView never sees tokens, keys or the database. Do not add a command that would hand it secrets.
- Logic goes in `src/lib/` with tests (`*.test.ts`; story-level suites are named `usNN.test.ts`).
- Check: `pnpm test` and `pnpm build` (runs `tsc --noEmit`).

## Backend (`src-tauri/`)

- Rust (1.88+), Tauri 2. One module per concern in `src-tauri/src/`; `commands.rs` is the boundary with the frontend.
- **Errors**: return `AppError` (`error.rs`). Each variant's `#[error(...)]` text is friendly Portuguese for the user; technical detail goes only to the log (and never contains user content). Add a variant instead of reusing a misleading one.
- **Validate everything from the frontend**: ids through the existing helpers (`account_id`, `checked_account_id`, `message::valid_account_id`), size and count limits as constants, `require_active` before anything that needs the server. A folder or message id that does not belong to the requested account must fail with `InvalidId`.
- **Secrets in memory** use `Zeroizing`; never derive or print `Debug` for keys. Secrets live in the OS keyring, never in the database in plain text or in files.
- **Database**: SQLCipher, migrations versioned by `PRAGMA user_version`. To change the schema, add `src-tauri/migrations/00NN_<name>.sql` and append it to the list in `db.rs`. **Never edit an existing migration.** Keep `temp_store = MEMORY`, `secure_delete` and the encrypted WAL. Update "Esquema do banco" in the threat model.
- **Third-party content**: HTML is cleaned by allowlist in Rust (`sanitize.rs`) on every read; remote images stay blocked until the user allows them; links only `http`/`https`/`mailto`, opened in the system browser, with tracking params stripped. Do not loosen any of this without a threat-model entry and tests (the sanitizer has an attack corpus and proptest).
- **Network**: TLS with certificate validation always; no new outbound host without documenting it in "O que sai do computador" (docs) and the threat model. Never send read receipts.
- **Dependencies**: justify new crates; they must pass `cargo deny check` (`deny.toml`). Keep `Cargo.lock` committed and builds `--locked`.
- `unsafe` only when unavoidable (Windows APIs), with a `// SAFETY:` comment.
- OAuth client ids come from `.env` (dev) or repository secrets (release) via `option_env!` in `provider.rs`; never hardcode them.
- Dev builds (`pnpm tauri dev`) use a separate data folder and keyring entries; keep it that way so they never touch an installed EpistulaMail.
- Windows build prerequisite: `./scripts/setup-build-tools.ps1` (portable Perl for SQLCipher's OpenSSL, into `.tools/`).
- Check (from `src-tauri/`): `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked`. Tests run against a real SQLCipher database. Add tests for every behavior change, especially security properties (leaks, injection, cross-account access).

## Landing page (`site/`)

- Static HTML/CSS/JS, no framework and no own dependencies. `build.mjs` builds `site/dist/` by copying `tokens.css`, `components.css`, the fonts from `@fontsource` and the DS sprite. Run locally with `cd site; node build.mjs --serve` (http://localhost:4321).
- `site.css` holds only site-specific styles, always with DS tokens, every class prefixed `site-`. Everything else is DS classes.
- App "screenshots" are real DS markup (same classes as the app) inside `.site-shot`, not images. Update them when the app UI changes.
- No third-party requests, cookies or tracking. Security headers and the CSP live in `site/vercel.json`; if you add a resource type, update the CSP deliberately.
- The download button and version come from the latest published GitHub release at build time; publishing a release redeploys the site.
- Legal pages (`privacidade.html`, `termos.html`, `conta-google.html`) must stay consistent with what the app actually does; change them in the same PR as the behavior.
- Deploy: `site.yml` on push to `main` (production) or PR (preview), via the Vercel CLI and a token. Vercel is not connected to GitHub; do not "fix" that.

## Documentation site (`docs-site/`)

- Astro Starlight with the DS theme. Commands (from `docs-site/`, own `pnpm-lock.yaml`): `pnpm dev`, `pnpm build`. Both run `scripts/sync-ds.mjs` first, which regenerates the DS copies (gitignored; never edit them).
- **Structure** (`astro.config.mjs`): two sidebar groups. "Comece aqui" holds step pages at the root of `src/content/docs/`. "Documentação" holds sections in `src/content/docs/guia/<section>/`, each autogenerated and collapsed; inside a section the sidebar shows only its sub-pages (`src/components/Sidebar.astro`). New root pages must be added to the sidebar config by slug with labels in all five languages (`t(pt, en, es, fr, it)`); new sections use `section(dir, ...labels)`.
- **Five languages, always together.** Portuguese is the root locale; translations live in `en/`, `es/`, `fr/`, `it/` with the **same file paths and slugs** (slugs stay in Portuguese). Every new or changed page must be written in pt-BR and translated into all four other languages in the same change, with internal links pointing to the same locale (`/en/guia/...`). UI strings of custom components go in `src/content/i18n/<lang>.json` (schema in `content.config.ts`).
- **App screens** in docs use `<AppPreview name="..." />`. Previews are DS HTML in `src/previews.ts`, rendered in an isolated iframe because `components.css` collides with Starlight. Their texts stay in Portuguese (the app UI only exists in Portuguese); the captions are translated in `captions`. Keep them in sync with the real UI.
- MDX pages import from `@astrojs/starlight/components` (`Steps`, `Aside`, `CardGrid`, `LinkCard`) and local components via `~/components/...` (`AppPreview`, `Timeline`). Do not import app CSS into pages.
- Content rules: write for users, not developers (contributor pages live under `guia/contribuir/`); state what happens behind the scenes and where data stays; be honest about limits (Gmail only for now, unsigned installer, Google verification pending). Keep versions, file names and sizes in sync with `CHANGELOG.md` and the latest release.
- When a feature ships or changes behavior, update the affected docs pages (all languages), `guia/versoes/historico.mdx` and the Timeline on the home page.
- Deploy: `docs.yml` on changes to `docs-site/**`, `tokens.css` or `components.css`.

## User stories and decisions

- A new feature starts as `userstory/00NN-<slug>.md` in Portuguese, following the existing format: story (Como / quero / para que), **1. Contexto**, **2. Critérios de aceite** (`CA1`, `CA2`...), **3. Interface** (draft, approved DS pieces with L numbers), **4. Decisões** (`D1`, `D2`..., marking what is still open).
- Do not implement an open decision ("Em aberto") without asking the user.
- Reference stories, criteria and decisions in code comments, tests and commits (`US10, CA5`, `D2 da US14`) the way existing code does.

## Releases

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. Add `## [X.Y.Z] - YYYY-MM-DD` to `CHANGELOG.md` (sections: Novidades, Mudou, Corrigido, Avisos), written for users.
3. Update the docs (version history, installer file name in examples) and both READMEs if needed.
4. The user pushes the `vX.Y.Z` tag. `release.yml` builds the signed NSIS installer and opens a **draft** release; a human publishes it. Never create tags or publish releases without being asked.

## Before you finish

- [ ] Visual change? It exists in the DS (draft approved, then incorporated) and uses tokens only.
- [ ] Frontend: `pnpm test` and `pnpm build` pass. Design system: `pnpm ds:check` passes.
- [ ] Backend: `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked` pass.
- [ ] Docs changed: pt-BR plus `en`, `es`, `fr`, `it`, and `pnpm build` in `docs-site/` passes.
- [ ] Site changed: `node build.mjs` in `site/` passes.
- [ ] Security-relevant: threat model updated and tests added.
- [ ] User-visible change: `CHANGELOG.md` (under the next version), docs and READMEs updated.
- [ ] No secrets, no user content in logs, no third-party requests.

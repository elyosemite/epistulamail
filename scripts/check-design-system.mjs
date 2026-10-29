// Confere o design system (v3 §4.3 e §12): tokens, temas, registro de páginas e métricas de saúde.
// Uso: node scripts/check-design-system.mjs [--metrics]
// Sai com código 1 se houver erro (variável sem token, tema escuro divergente, página fora do registro).
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const ds = join(root, "design-system");
const read = (f) => readFileSync(join(ds, f), "utf8");
const errors = [];
const warnings = [];

const tokensCss = read("tokens.css");
const componentsCss = read("components.css");
const docsCss = read("docs.css");

/** Remove comentários e devolve o texto do bloco que começa na linha `start` (até a chave que fecha). */
function blocks(css) {
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const out = [];
  const re = /([^{}]+)\{/g;
  let m;
  while ((m = re.exec(clean))) {
    let depth = 1, i = re.lastIndex;
    while (depth && i < clean.length) { if (clean[i] === "{") depth++; else if (clean[i] === "}") depth--; i++; }
    out.push({ selector: m[1].trim(), body: clean.slice(re.lastIndex, i - 1) });
    re.lastIndex = i;
  }
  return out;
}
function decls(body) {
  const map = new Map();
  for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) map.set(m[1], m[2].trim());
  return map;
}

// ---- 1. tokens por tema ----
const top = blocks(tokensCss);
const light = new Map();
let darkMedia = new Map();
const darkForced = new Map();
for (const b of top) {
  if (b.selector === ":root") for (const [k, v] of decls(b.body)) light.set(k, v);
  else if (b.selector.startsWith("@media (prefers-color-scheme: dark)")) {
    for (const inner of blocks(b.body)) for (const [k, v] of decls(inner.body)) darkMedia.set(k, v);
  } else if (b.selector === ':root[data-theme="dark"]') for (const [k, v] of decls(b.body)) darkForced.set(k, v);
}
for (const [k, v] of darkMedia) if (darkForced.get(k) !== v) errors.push(`tema escuro diverge: ${k} (sistema: ${v} · forçado: ${darkForced.get(k) ?? "ausente"})`);
for (const k of darkForced.keys()) if (!darkMedia.has(k)) errors.push(`tema escuro diverge: ${k} só existe no bloco forçado`);
for (const k of darkMedia.keys()) if (!light.has(k)) errors.push(`token escuro sem token claro: ${k}`);

// ---- 2. var() sem definição ----
const files = { "tokens.css": tokensCss, "components.css": componentsCss, "docs.css": docsCss };
// Variáveis que o app define em tempo de execução (inline style ou JS); não existem em nenhum arquivo CSS.
const RUNTIME_VARS = ["--app-background", "--app-shell-list", "--swatch", "--x", "--y", "--lb-fg"];
const localDecl = new Set(RUNTIME_VARS);
for (const css of Object.values(files)) for (const m of css.matchAll(/(--[\w-]+)\s*:/g)) localDecl.add(m[1]);
const used = new Map();
for (const [name, css] of Object.entries(files)) {
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const m of clean.matchAll(/var\(\s*(--[\w-]+)/g)) {
    used.set(m[1], (used.get(m[1]) ?? 0) + 1);
    if (!localDecl.has(m[1])) errors.push(`${name}: var(${m[1]}) não está definida em nenhum lugar`);
  }
}

// ---- 2b. breakpoints: os @media do components.css têm de bater com os tokens --breakpoint-* ----
const bp = [...light].filter(([k]) => k.startsWith("--breakpoint-")).map(([, v]) => parseInt(v, 10));
const okWidths = new Set(bp.flatMap((n) => [n, n - 1]));
for (const m of componentsCss.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/@media\s*\((?:max|min)-width:\s*(\d+)px\)/g)) {
  if (!okWidths.has(Number(m[1]))) errors.push(`components.css: @media com ${m[1]}px não corresponde a nenhum token --breakpoint-* (${bp.join(", ")}px)`);
}

// ---- 3. páginas e registro ----
const docsJs = read("docs.js");
const registry = (0, eval)(docsJs.match(/const EPISTULA_PAGES = (\[[\s\S]*?\n\]);/)[1]);
const STATUS = new Set(["draft", "experimental", "beta", "stable", "deprecated", "removed"]);
const ids = [];
for (const g of registry) for (const [id, title, desc, status] of g.items) {
  ids.push(id);
  if (!existsSync(join(ds, `${id}.html`))) errors.push(`registro: ${id}.html não existe`);
  else if (!read(`${id}.html`).includes(`data-page="${id}"`)) errors.push(`${id}.html: falta data-page="${id}"`);
  if (!STATUS.has(status)) errors.push(`registro: ${id} sem status válido (${status})`);
  if (!title || !desc) errors.push(`registro: ${id} sem título ou descrição`);
}
for (const f of readdirSync(ds).filter((n) => n.endsWith(".html") && n !== "index.html" && !n.startsWith("rascunho"))) {
  if (!ids.includes(f.slice(0, -5))) errors.push(`${f} não está em EPISTULA_PAGES`);
}

// ---- 3b. ligação entre o app e o design system (v3 §12: drift e adoção) ----
// Todo componente de src/ começa com um comentário que aponta para a página do DS (`// DS: botoes.html · ...`).
let appFiles = 0, appLinked = 0;
const walkTsx = (d) => {
  for (const e of readdirSync(d, { withFileTypes: true })) {
    const p = join(d, e.name);
    if (e.isDirectory()) walkTsx(p);
    else if (e.name.endsWith(".tsx")) {
      appFiles++;
      const head = readFileSync(p, "utf8").split("\n").slice(0, 14).filter((l) => l.trim().startsWith("//")).join("\n");
      const refs = [...head.matchAll(/([a-z][a-z-]*)\.html/g)].map((m) => m[1]);
      if (refs.length) appLinked++;
      for (const r of refs) if (!existsSync(join(ds, `${r}.html`))) errors.push(`${p.replace(root, "").replace(/\\/g, "/").slice(1)}: aponta para ${r}.html, que não existe no design system`);
    }
  }
};
if (existsSync(join(root, "src"))) walkTsx(join(root, "src"));

// ---- 4. métricas (v3 §12) ----
const clean = componentsCss.replace(/\/\*(?![^*]*físico)[\s\S]*?\*\//g, "");
const declRe = /(^|[;{])\s*([a-z-]+)\s*:\s*([^;{}]+)(?=[;}])/g;
const VISUAL = /^(color|background(-color)?|border(-color)?|border(-top|-right|-bottom|-left|-inline-start|-inline-end)?(-color)?|outline(-color)?|box-shadow|fill|stroke|font(-family|-size|-weight)?|padding(-\w+)*|margin(-\w+)*|gap|row-gap|column-gap|border-radius|z-index|transition(-duration)?|animation-duration)$/;
let visual = 0, tokenized = 0, hardcoded = [];
for (const m of clean.matchAll(declRe)) {
  const prop = m[2], value = m[3].trim();
  if (prop.startsWith("--") || !VISUAL.test(prop)) continue;
  if (/^(0|none|inherit|initial|auto|transparent|currentColor|unset|normal|revert)$/i.test(value)) continue;
  visual++;
  if (/var\(/.test(value)) tokenized++;
  else if (/#[0-9a-f]{3,8}\b|rgba?\(|\b\d*\.?\d+(px|rem|em|ms|s)\b/i.test(value)) hardcoded.push(`${prop}: ${value}`);
}
const physical = (clean.split("\n").filter((l) => !l.includes("físico")).join("\n").match(/(margin|padding)-(left|right)\s*:|(^|[;{\s])(left|right)\s*:|text-align\s*:\s*(left|right)|border-(left|right)(-\w+)?\s*:|border-(top|bottom)-(left|right)-radius\s*:/g) ?? []).length;
const allTokenNames = [...light.keys()];
const primitives = allTokenNames.filter((n) => n.startsWith("--palette-"));
const semantic = allTokenNames.filter((n) => !n.startsWith("--palette-"));
const corpus = [componentsCss, docsCss, docsJs, ...readdirSync(ds).filter((n) => n.endsWith(".html")).map(read)].join("\n");
let src = "";
for (const dir of ["src", "site", "docs-site/src"]) {
  const walk = (d) => { for (const e of readdirSync(d, { withFileTypes: true })) { if (e.name === "node_modules" || e.name === "dist") continue; const p = join(d, e.name); if (e.isDirectory()) walk(p); else if (/\.(tsx?|css|html|astro|mdx?|mjs)$/.test(e.name)) src += readFileSync(p, "utf8") + "\n"; } };
  if (existsSync(join(root, dir))) walk(join(root, dir));
}
const tokensBody = tokensCss.replace(/\/\*[\s\S]*?\*\//g, "");
const refs = (n, text) => text.includes(`var(${n})`) || text.includes(`var(${n},`);
// Primitivo: tem de ser usado por algum semântico. Semântico: por componentes, páginas, app, site ou docs (ou por outro token).
const orphanPrimitives = primitives.filter((n) => !refs(n, tokensBody) && !primitives.some((o) => o.startsWith(n + "-a"))); // a base de uma opacidade (-aNN) conta como usada
const orphans = semantic.filter((n) => !refs(n, corpus) && !refs(n, src) && !refs(n, tokensBody) && !/^--(gradient|color-canvas-|breakpoint-)/.test(n));
// Regra de ouro dos três níveis: nada fora de tokens.css usa a paleta direto.
for (const [name, text] of [["components.css", componentsCss], ["docs.css", docsCss], ["docs.js", docsJs], ["src/ (app)", src]]) {
  const bad = [...new Set([...text.matchAll(/var\(\s*(--palette-[\w-]+)/g)].map((m) => m[1]))];
  if (bad.length) errors.push(`${name} usa primitivo(s) direto (use o token semântico): ${bad.slice(0, 5).join(", ")}`);
}
const darkMissing = semantic.filter((n) => n.startsWith("--color-") && !darkMedia.has(n));

const pct = (a, b) => (b ? Math.round((a / b) * 1000) / 10 : 100);
const report = {
  tokens: allTokenNames.length,
  primitivos: primitives.length,
  semanticos: semantic.length,
  tokensComTemaEscuro: [...darkMedia.keys()].length,
  paginas: ids.length,
  arquivosDoAppComPaginaDoDS: `${appLinked} de ${appFiles} (${pct(appLinked, appFiles)}%)`,
  paginasPorStatus: Object.fromEntries([...STATUS].map((s) => [s, registry.flatMap((g) => g.items).filter((i) => i[3] === s).length])),
  coberturaDeTokens: `${pct(tokenized, visual)}% (${tokenized} de ${visual} declarações visuais em components.css)`,
  valoresFixosEmComponents: hardcoded.length,
  propriedadesFisicas: physical,
  tokensSemUso: orphans.length,
  primitivosSemUso: orphanPrimitives.length,
};
if (hardcoded.length) warnings.push(`${hardcoded.length} valor(es) fixo(s) em components.css (não passam por token)`);
if (physical) warnings.push(`${physical} propriedade(s) física(s) (left/right) em components.css; prefira as lógicas (inline-start/end)`);
if (orphanPrimitives.length) warnings.push(`${orphanPrimitives.length} primitivo(s) sem nenhum semântico apontando: ${orphanPrimitives.slice(0, 8).join(", ")}`);
if (orphans.length) warnings.push(`${orphans.length} token(s) sem uso: ${orphans.slice(0, 12).join(", ")}${orphans.length > 12 ? "…" : ""}`);
if (darkMissing.length) warnings.push(`${darkMissing.length} token(s) de cor sem valor próprio no escuro (herdam o claro): ${darkMissing.slice(0, 8).join(", ")}${darkMissing.length > 8 ? "…" : ""}`);

if (process.argv.includes("--metrics")) {
  console.log(JSON.stringify(report, null, 2));
  if (process.argv.includes("--list")) { console.log("\nvalores fixos:\n" + hardcoded.join("\n")); console.log("\nsem uso:\n" + orphans.join("\n")); console.log("\nsem escuro:\n" + darkMissing.join("\n")); }
}
for (const w of warnings) console.warn("aviso:", w);
for (const e of errors) console.error("erro:", e);
console.log(errors.length ? `\n${errors.length} erro(s).` : `\nDesign system ok: ${report.tokens} tokens, ${report.paginas} páginas, cobertura de tokens ${report.coberturaDeTokens.split(" ")[0]}.`);
process.exit(errors.length ? 1 : 0);

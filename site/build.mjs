// Build do site (landing page) sem dependências: monta `dist/` com a página, o CSS do site, os arquivos
// do design system (tokens.css e components.css, copiados de ../design-system, nunca editados aqui), as
// fontes empacotadas (nenhuma requisição a terceiros) e o sprite de ícones extraído de docs.js, como no app.
//   node build.mjs          → dist/
//   node build.mjs --serve  → dist/ e um servidor local em http://localhost:4321
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const ds = join(root, "design-system");
const dist = join(here, "dist");

function fail(message) {
  console.error(`build: ${message}`);
  process.exit(1);
}

/** O sprite do DS vive embutido em docs.js (decisão L2); o build falha se o formato mudar. */
function iconSprite() {
  const source = readFileSync(join(ds, "docs.js"), "utf8");
  const match = source.match(/insertAdjacentHTML\('afterbegin', ("(?:[^"\\]|\\.)*<symbol(?:[^"\\]|\\.)*")\)/);
  if (!match) fail("sprite de ícones não encontrado em design-system/docs.js");
  return JSON.parse(match[1]);
}

const FONTS = [
  ["@fontsource/inter/files/inter-latin-400-normal.woff2", "Inter", 400],
  ["@fontsource/inter/files/inter-latin-500-normal.woff2", "Inter", 500],
  ["@fontsource/inter/files/inter-latin-600-normal.woff2", "Inter", 600],
  ["@fontsource/inter/files/inter-latin-700-normal.woff2", "Inter", 700],
  ["@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2", "JetBrains Mono", 400],
  ["@fontsource/jetbrains-mono/files/jetbrains-mono-latin-500-normal.woff2", "JetBrains Mono", 500],
];

function build() {
  rmSync(dist, { recursive: true, force: true });
  mkdirSync(join(dist, "ds"), { recursive: true });
  mkdirSync(join(dist, "fonts"), { recursive: true });

  for (const file of ["tokens.css", "components.css"]) copyFileSync(join(ds, file), join(dist, "ds", file));

  const faces = [];
  for (const [path, family, weight] of FONTS) {
    const from = join(root, "node_modules", path);
    if (!existsSync(from)) fail(`fonte ${path} não encontrada: rode "pnpm install" na raiz do repositório`);
    const name = path.split("/").pop();
    copyFileSync(from, join(dist, "fonts", name));
    faces.push(`@font-face { font-family: "${family}"; font-style: normal; font-weight: ${weight}; font-display: swap; src: url("${name}") format("woff2"); }`);
  }
  writeFileSync(join(dist, "fonts", "fonts.css"), faces.join("\n") + "\n");

  for (const file of ["site.css", "site.js", "theme.js"]) copyFileSync(join(here, file), join(dist, file));

  const page = readFileSync(join(here, "index.html"), "utf8");
  if (!page.includes("<!-- ds-icons -->")) fail("index.html sem o marcador <!-- ds-icons -->");
  writeFileSync(join(dist, "index.html"), page.replace("<!-- ds-icons -->", `<div hidden>${iconSprite()}</div>`));
  console.log("build: dist/ pronto");
}

const TYPES = { ".html": "text/html; charset=utf-8", ".css": "text/css", ".js": "text/javascript", ".woff2": "font/woff2" };

function serve() {
  createServer((req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^([/\\])+/, "");
    const file = join(dist, path === "" || path === "." ? "index.html" : path);
    if (!file.startsWith(dist) || !existsSync(file)) {
      res.writeHead(404).end("não encontrado");
      return;
    }
    res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" }).end(readFileSync(file));
  }).listen(4321, () => console.log("build: http://localhost:4321"));
}

build();
if (process.argv.includes("--serve")) serve();

// Build do site (landing page) sem dependências: monta `dist/` com a página, o CSS do site, os arquivos
// do design system (tokens.css e components.css, copiados de ../design-system, nunca editados aqui), as
// fontes empacotadas (nenhuma requisição a terceiros) e o sprite de ícones extraído de docs.js, como no app.
//   node build.mjs          → dist/
//   node build.mjs --serve  → dist/ e um servidor local em http://localhost:4321
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
  ["@fontsource/inter/files/inter-latin-400-normal.woff2", "Inter", 400, "normal"],
  ["@fontsource/inter/files/inter-latin-500-normal.woff2", "Inter", 500, "normal"],
  ["@fontsource/inter/files/inter-latin-600-normal.woff2", "Inter", 600, "normal"],
  ["@fontsource/inter/files/inter-latin-700-normal.woff2", "Inter", 700, "normal"],
  ["@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2", "JetBrains Mono", 400, "normal"],
  ["@fontsource/jetbrains-mono/files/jetbrains-mono-latin-500-normal.woff2", "JetBrains Mono", 500, "normal"],
  // O acento editorial do herói: a mesma mono, em itálico. Já vem no pacote; nada de fonte remota.
  ["@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-italic.woff2", "JetBrains Mono", 400, "italic"],
];

const REPO = "elyosemite/epistulamail";

/**
 * A última release publicada (rascunhos não contam), para o botão de download e a versão do site. O
 * botão aponta para o pacote do sistema, com o menu para os dois. Sem rede ou sem release publicada,
 * usa a versão do package.json e os nomes que o release.yml dá aos arquivos.
 */
async function latestRelease() {
  const page = `https://github.com/${REPO}/releases`;
  const fallback = () => {
    const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
    const base = `${page}/download/v${version}`;
    return {
      windows: `${base}/EpistulaMail_${version}_x64-setup.exe`,
      linuxAppImage: `${base}/EpistulaMail_${version}_amd64.AppImage`,
      linuxDeb: `${base}/EpistulaMail_${version}_amd64.deb`,
      page: `${page}/tag/v${version}`,
      label: `Versão ${version}`,
    };
  };
  try {
    const headers = { accept: "application/vnd.github+json", "user-agent": "epistula-site-build" };
    if (process.env.GITHUB_TOKEN) headers.authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers, signal: AbortSignal.timeout(10_000) });
    if (!res.ok) {
      console.warn(`build: sem release publicada (${res.status}); usando a versão do package.json`);
      return fallback();
    }
    const release = await res.json();
    const exe = release.assets?.find((a) => a.name.endsWith("-setup.exe"));
    const appImage = release.assets?.find((a) => a.name.endsWith(".AppImage"));
    const deb = release.assets?.find((a) => a.name.endsWith(".deb"));
    return {
      windows: exe?.browser_download_url ?? fallback().windows,
      // Uma release antiga pode não ter os pacotes Linux: o menu do site esconde o que não existe.
      linuxAppImage: appImage?.browser_download_url ?? "",
      linuxDeb: deb?.browser_download_url ?? "",
      page: release.html_url,
      label: `Versão ${String(release.tag_name).replace(/^v/, "")}`,
    };
  } catch (error) {
    console.warn(`build: não foi possível consultar a release (${error.message}); usando a versão do package.json`);
    return fallback();
  }
}

function withRelease(html, release) {
  const escape = (v) => v.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
  return html.replace(/\{\{release\.(windows|linuxAppImage|linuxDeb|page|label)\}\}/g, (_, key) => escape(release[key]));
}

async function build() {
  const release = await latestRelease();
  rmSync(dist, { recursive: true, force: true });
  mkdirSync(join(dist, "ds"), { recursive: true });
  mkdirSync(join(dist, "fonts"), { recursive: true });

  for (const file of ["tokens.css", "components.css"]) copyFileSync(join(ds, file), join(dist, "ds", file));

  const faces = [];
  for (const [path, family, weight, style] of FONTS) {
    const from = join(root, "node_modules", path);
    if (!existsSync(from)) fail(`fonte ${path} não encontrada: rode "pnpm install" na raiz do repositório`);
    const name = path.split("/").pop();
    copyFileSync(from, join(dist, "fonts", name));
    faces.push(`@font-face { font-family: "${family}"; font-style: ${style}; font-weight: ${weight}; font-display: swap; src: url("${name}") format("woff2"); }`);
  }
  writeFileSync(join(dist, "fonts", "fonts.css"), faces.join("\n") + "\n");

  for (const file of ["site.css", "site.js", "ascii.js", "theme.js"]) copyFileSync(join(here, file), join(dist, file));

  // A janela do app recortada da captura do README (docs/images), sem o papel de parede.
  mkdirSync(join(dist, "images"), { recursive: true });
  copyFileSync(join(root, "docs", "images", "epistula-janela.webp"), join(dist, "images", "epistula-janela.webp"));
  // Retrato do Yuri, enviado por ele para o anúncio e a seção "Quem está por trás".
  copyFileSync(join(here, "images", "yuri-melo.jpg"), join(dist, "images", "yuri-melo.jpg"));

  // A logo do EpistulaMail (favicon, marca no topo e no rodapé), gerada da mesma imagem dos ícones do app.
  mkdirSync(join(dist, "images", "logo"), { recursive: true });
  for (const name of readdirSync(join(here, "images", "logo"))) copyFileSync(join(here, "images", "logo", name), join(dist, "images", "logo", name));

  // Fotos dos avatares (domínio público, créditos em images/people/CREDITS.md).
  mkdirSync(join(dist, "images", "people"), { recursive: true });
  for (const name of readdirSync(join(here, "images", "people")).filter((n) => n.endsWith(".webp"))) {
    copyFileSync(join(here, "images", "people", name), join(dist, "images", "people", name));
  }

  // A página inicial, a política de privacidade, os termos de uso e como o app usa a conta Google (verificação do OAuth).
  const sprite = `<div hidden>${iconSprite()}</div>`;
  for (const file of ["index.html", "privacidade.html", "termos.html", "conta-google.html", "blog.html", "blog-anuncio.html", "blog-banco-cifrado.html", "blog-imagens-remotas.html", "blog-linux.html", "blog-atualizacao-automatica.html", "blog-contas-microsoft-e-imap.html", "blog-design-system.html"]) {
    const page = readFileSync(join(here, file), "utf8");
    if (!page.includes("<!-- ds-icons -->")) fail(`${file} sem o marcador <!-- ds-icons -->`);
    const html = withRelease(page.replace("<!-- ds-icons -->", sprite), release);
    if (html.includes("{{release.")) fail(`${file} com marcador de release desconhecido`);
    writeFileSync(join(dist, file), html);
  }
  console.log(`build: dist/ pronto (${release.label}, ${release.windows})`);
}

const TYPES = { ".html": "text/html; charset=utf-8", ".css": "text/css", ".js": "text/javascript", ".woff2": "font/woff2", ".png": "image/png", ".webp": "image/webp", ".ico": "image/x-icon" };

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

await build();
if (process.argv.includes("--serve")) serve();

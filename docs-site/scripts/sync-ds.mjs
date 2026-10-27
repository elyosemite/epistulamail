// Traz do design system o que a documentação usa:
// - src/styles/ds-tokens.css: os tokens (tokens.css inteiro), para o tema do Starlight;
// - src/styles/ds-pieces.css: o trecho de components.css entre @docs-site:start e @docs-site:end (brilho do topo,
//   título em gradiente, ícones de marca, idioma e timeline). O resto do components.css não entra nas páginas: os
//   nomes de classe do app colidem com os do Starlight;
// - public/ds/: tokens.css, components.css e as fontes, para as telas do app mostradas nas páginas (AppPreview), que
//   ficam num iframe isolado e aí podem usar o components.css inteiro;
// - src/generated/sprite.html: o sprite de ícones do DS (o mesmo que o app e o site usam).
// Tudo é gerado e fica fora do git; os scripts do package.json rodam isto antes do dev e do build.
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const site = join(here, "..");
const ds = join(site, "..", "design-system");
const styles = join(site, "src", "styles");
const generated = join(site, "src", "generated");
const pub = join(site, "public", "ds");
const header = "/* Gerado por docs-site/scripts/sync-ds.mjs a partir de design-system/. Não edite aqui. */\n";

function fail(message) {
  console.error(`sync-ds: ${message}`);
  process.exit(1);
}

mkdirSync(generated, { recursive: true });
mkdirSync(join(pub, "fonts"), { recursive: true });

const tokens = readFileSync(join(ds, "tokens.css"), "utf8");
const components = readFileSync(join(ds, "components.css"), "utf8");
writeFileSync(join(styles, "ds-tokens.css"), header + tokens);

const pieces = components.match(/\/\* @docs-site:start \*\/([\s\S]*?)\/\* @docs-site:end \*\//);
if (!pieces) fail("trecho @docs-site não encontrado em design-system/components.css");
writeFileSync(join(styles, "ds-pieces.css"), header + pieces[1].trim() + "\n");

// Telas do app: o DS inteiro, num iframe.
writeFileSync(join(pub, "tokens.css"), header + tokens);
writeFileSync(join(pub, "components.css"), header + components);
const faces = [];
for (const [pkg, family, weight] of [
  ["inter", "Inter", 400],
  ["inter", "Inter", 500],
  ["inter", "Inter", 600],
  ["inter", "Inter", 700],
  ["jetbrains-mono", "JetBrains Mono", 400],
]) {
  const name = `${pkg}-latin-${weight}-normal.woff2`;
  copyFileSync(join(site, "node_modules", "@fontsource", pkg, "files", name), join(pub, "fonts", name));
  faces.push(`@font-face { font-family: "${family}"; font-weight: ${weight}; font-display: swap; src: url("fonts/${name}") format("woff2"); }`);
}
writeFileSync(join(pub, "fonts.css"), faces.join("\n") + "\n");

const docsJs = readFileSync(join(ds, "docs.js"), "utf8");
const sprite = docsJs.match(/insertAdjacentHTML\('afterbegin', ("(?:[^"\\]|\\.)*<symbol(?:[^"\\]|\\.)*")\)/);
if (!sprite) fail("sprite de ícones não encontrado em design-system/docs.js");
writeFileSync(join(generated, "sprite.html"), JSON.parse(sprite[1]));

console.log("sync-ds: tokens, peças, telas do app e ícones copiados");

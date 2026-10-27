// Traz do design system o que a documentação usa: os tokens (tokens.css inteiro) e as peças de documentação
// (o trecho de components.css entre @docs-site:start e @docs-site:end: brilho do topo, título em gradiente, ícones
// de marca, idioma e timeline). O resto do components.css não entra: os nomes de classe do app colidem com os do
// Starlight. Os arquivos gerados ficam fora do git; rode antes do dev e do build (os scripts do package.json já rodam).
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const ds = join(here, "..", "..", "design-system");
const out = join(here, "..", "src", "styles");
const header = "/* Gerado por docs-site/scripts/sync-ds.mjs a partir de design-system/. Não edite aqui. */\n";

writeFileSync(join(out, "ds-tokens.css"), header + readFileSync(join(ds, "tokens.css"), "utf8"));

const components = readFileSync(join(ds, "components.css"), "utf8");
const match = components.match(/\/\* @docs-site:start \*\/([\s\S]*?)\/\* @docs-site:end \*\//);
if (!match) {
  console.error("sync-ds: trecho @docs-site não encontrado em design-system/components.css");
  process.exit(1);
}
writeFileSync(join(out, "ds-pieces.css"), header + match[1].trim() + "\n");
console.log("sync-ds: tokens e peças de documentação copiados");

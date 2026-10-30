import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { Plugin } from "vite";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

const designSystemDir = fileURLToPath(new URL("./design-system/", import.meta.url));

/**
 * O sprite de ícones do design system vive embutido em docs.js (decisão L2: não haverá icons.svg).
 * Este plugin extrai o <svg> de lá em tempo de build, para que o app nunca tenha uma cópia
 * própria dos ícones. O build falha se o formato de docs.js mudar e o sprite não for encontrado.
 */
function designSystemIcons(): Plugin {
  const id = "virtual:ds-icons";
  const resolved = "\0" + id;
  const source = designSystemDir + "docs.js";
  return {
    name: "epistula-ds-icons",
    resolveId: (request) => (request === id ? resolved : undefined),
    load(request) {
      if (request !== resolved) return;
      this.addWatchFile(source);
      const match = readFileSync(source, "utf8").match(/insertAdjacentHTML\('afterbegin', ("(?:[^"\\]|\\.)*<symbol(?:[^"\\]|\\.)*")\)/);
      if (!match) this.error("Sprite de ícones não encontrado em design-system/docs.js");
      const sprite: string = JSON.parse(match[1]);
      const names = [...sprite.matchAll(/<symbol id="i-([a-z0-9-]+)"/g)].map((m) => m[1]);
      return `export const sprite = ${JSON.stringify(sprite)};\nexport const iconNames = ${JSON.stringify(names)};`;
    },
  };
}

export default defineConfig({
  plugins: [react(), designSystemIcons()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Pastas grandes que o app não importa: observá-las travava o Vite por minutos na primeira carga
    // (.tools tem o Strawberry Perl portátil do build do OpenSSL, com ~20 mil arquivos).
    watch: { ignored: ["**/src-tauri/**", "**/.tools/**", "**/dist/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
  test: {
    environment: "node",
    setupFiles: ["./vitest.setup.ts"],
    testTimeout: 15_000,
    hookTimeout: 15_000,
  },
});

import type { Appearance, CanvasChoice, ThemeChoice } from "./tauri";

/** Cores do fundo, na ordem da paleta (US6, CA4). O valor é o sufixo do token `--color-canvas-*`. */
export const CANVAS_CHOICES: ReadonlyArray<{ value: CanvasChoice; label: string }> = [
  { value: "", label: "Padrão" },
  { value: "blue", label: "Azul" },
  { value: "teal", label: "Verde-azulado" },
  { value: "green", label: "Verde" },
  { value: "purple", label: "Roxo" },
  { value: "magenta", label: "Magenta" },
  { value: "orange", label: "Laranja" },
  { value: "sand", label: "Areia" },
];

export const THEME_CHOICES: ReadonlyArray<{ value: ThemeChoice; label: string; description: string; icon: string }> = [
  { value: "light", label: "Claro", description: "Sempre claro", icon: "sun" },
  { value: "dark", label: "Escuro", description: "Sempre escuro", icon: "moon" },
  { value: "system", label: "Sistema", description: "Segue o Windows", icon: "monitor" },
];

export const DEFAULT_APPEARANCE: Appearance = { theme: "system", canvas: "" };

export const canvasToken = (canvas: CanvasChoice) => (canvas ? `var(--color-canvas-${canvas})` : "var(--color-canvas)");

/** Valor salvo (banco ou cache local) só é aceito se for uma opção conhecida. */
export function parseAppearance(value: unknown): Appearance {
  const v = (value ?? {}) as Partial<Record<keyof Appearance, unknown>>;
  const theme = THEME_CHOICES.some((t) => t.value === v.theme) ? (v.theme as ThemeChoice) : DEFAULT_APPEARANCE.theme;
  const canvas = CANVAS_CHOICES.some((c) => c.value === v.canvas) ? (v.canvas as CanvasChoice) : DEFAULT_APPEARANCE.canvas;
  return { theme, canvas };
}

const CACHE_KEY = "epistula-appearance";

/**
 * Aplica no `<html>`: Sistema remove `data-theme` (segue `prefers-color-scheme`); Claro e Escuro o
 * fixam. O fundo vai para `--app-background`, que a casca em blocos lê. Guarda uma cópia local para a
 * próxima abertura pintar certo antes da resposta do banco.
 */
export function applyAppearance(appearance: Appearance, root: HTMLElement = document.documentElement): void {
  if (appearance.theme === "system") delete root.dataset.theme;
  else root.dataset.theme = appearance.theme;
  if (appearance.canvas) root.style.setProperty("--app-background", canvasToken(appearance.canvas));
  else root.style.removeProperty("--app-background");
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(appearance));
  } catch {
    // Sem armazenamento local: o banco continua sendo a fonte.
  }
}

/** Aparência da última abertura (cópia local), para aplicar antes do primeiro desenho do React. */
export function cachedAppearance(): Appearance {
  try {
    return parseAppearance(JSON.parse(localStorage.getItem(CACHE_KEY) ?? "null"));
  } catch {
    return DEFAULT_APPEARANCE;
  }
}

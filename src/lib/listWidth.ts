/** Largura da coluna de e-mails (US4, CA2), em pixels. Limites do DS: 20rem a 35rem. */
export const LIST_MIN = 320;
export const LIST_MAX = 560;
export const LIST_DEFAULT = 400;
export const STEP = 16;
export const BIG_STEP = 64;

export function clampWidth(px: number): number {
  return Math.round(Math.max(LIST_MIN, Math.min(LIST_MAX, px)));
}

/** Nova largura para uma tecla no divisor, ou `null` se a tecla não ajusta. */
export function keyWidth(current: number, key: string, shift: boolean): number | null {
  const step = shift ? BIG_STEP : STEP;
  switch (key) {
    case "ArrowLeft":
      return clampWidth(current - step);
    case "ArrowRight":
      return clampWidth(current + step);
    case "Home":
      return LIST_MIN;
    case "End":
      return LIST_MAX;
    default:
      return null;
  }
}

/** Posição entre os limites, em porcentagem (anunciada pelo leitor de tela). */
export function widthPercent(px: number): number {
  return Math.round(((clampWidth(px) - LIST_MIN) / (LIST_MAX - LIST_MIN)) * 100);
}

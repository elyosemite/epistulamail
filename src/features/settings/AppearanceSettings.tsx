// Composição de DS: modais.html "Configurações" (.settings__group) + selecao.html (.theme-picker; .color-picker com
// .color-swatch__fill)
import { CANVAS_CHOICES, canvasToken, THEME_CHOICES } from "../../lib/appearance";
import type { Appearance } from "../../lib/tauri";
import { CanvasPicker } from "../../ui/CanvasPicker";
import { SettingNote, SettingsGroup } from "../../ui/SettingsSheet";
import { ThemePicker } from "../../ui/ThemePicker";

const CANVAS_OPTIONS = CANVAS_CHOICES.map((c) => ({ ...c, swatch: canvasToken(c.value) }));

/** Aparência (US6, CA3–CA4): tema e cor do fundo, aplicados na hora. */
export function AppearanceSettings({ appearance, onChange }: { appearance: Appearance; onChange: (change: Partial<Appearance>) => void }) {
  return (
    <>
      <SettingsGroup title="Tema">
        <ThemePicker legend="Tema" options={THEME_CHOICES} value={appearance.theme} onChange={(theme) => onChange({ theme })} />
      </SettingsGroup>
      <SettingsGroup title="Cor do fundo">
        <CanvasPicker legend="Cor do fundo" options={CANVAS_OPTIONS} value={appearance.canvas} onChange={(canvas) => onChange({ canvas })} />
        <SettingNote>A cor aparece atrás dos blocos e na coluna de contas, e acompanha o tema.</SettingNote>
      </SettingsGroup>
    </>
  );
}

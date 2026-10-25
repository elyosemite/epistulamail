// Composição de DS: modais.html "Configurações" (.settings-sheet sobre as duas colunas, D2) + vazios.html (.empty
// "Em breve" nas seções ainda sem opções)
import { useState } from "react";
import type { Account, Appearance } from "../../lib/tauri";
import { EmptyState } from "../../ui/EmptyState";
import { SettingsSheet, SettingsTitle, type SettingsSection } from "../../ui/SettingsSheet";
import { AppearanceSettings } from "./AppearanceSettings";
import { GeneralSettings } from "./GeneralSettings";
import { SyncSettings } from "./SyncSettings";

const SECTIONS: SettingsSection[] = [
  { id: "general", label: "Geral", icon: "sliders" },
  { id: "appearance", label: "Aparência", icon: "palette" },
  { id: "account", label: "Conta", icon: "user" },
  { id: "email", label: "E-mail", icon: "mail" },
];

const SOON: Record<string, string> = {
  account: "Nome, cor e foto de cada conta, assinatura e sair da conta.",
};

interface SettingsProps {
  accounts: Account[];
  appearance: Appearance;
  onAppearance: (change: Partial<Appearance>) => void;
  onClose: () => void;
}

/** Configurações (US5, CA6): seções à esquerda, opções da seção à direita. Abrem sempre em Geral (US7, CA4). */
export function Settings({ accounts, appearance, onAppearance, onClose }: SettingsProps) {
  const [section, setSection] = useState("general");
  const current = SECTIONS.find((s) => s.id === section)!;
  return (
    <SettingsSheet sections={SECTIONS} current={section} onSection={setSection} onClose={onClose}>
      <SettingsTitle>{current.label}</SettingsTitle>
      {section === "general" ? (
        <GeneralSettings appearance={appearance} onChange={onAppearance} />
      ) : section === "email" ? (
        <SyncSettings accounts={accounts} />
      ) : section === "appearance" ? (
        <AppearanceSettings appearance={appearance} onChange={onAppearance} />
      ) : (
        <EmptyState title="Em breve" description={SOON[section]} />
      )}
    </SettingsSheet>
  );
}

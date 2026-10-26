// Composição de DS: modais.html "Configurações" (.settings-sheet sobre as duas colunas, D2) + vazios.html (.empty
// "Em breve" nas seções ainda sem opções)
import { useState } from "react";
import type { Account, Appearance } from "../../lib/tauri";
import { EmptyState } from "../../ui/EmptyState";
import { SettingsSheet, SettingsTitle, type SettingsSection } from "../../ui/SettingsSheet";
import { AppearanceSettings } from "./AppearanceSettings";
import { GeneralSettings } from "./GeneralSettings";
import { NotificationSettings } from "./NotificationSettings";
import { OfflineSettings } from "./OfflineSettings";
import { PrivacySettings } from "./PrivacySettings";
import { RulesSettings, type RulesRequest } from "../rules/RulesSettings";
import { SyncSettings } from "./SyncSettings";

const SECTIONS: SettingsSection[] = [
  { id: "general", label: "Geral", icon: "sliders" },
  { id: "appearance", label: "Aparência", icon: "palette" },
  { id: "notifications", label: "Notificações", icon: "bell" },
  { id: "privacy", label: "Privacidade", icon: "shield" },
  { id: "rules", label: "Regras", icon: "filter" },
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
  /** Seção ao abrir; sem ela, Geral. O sino abre em Notificações (US8, CA7). */
  initialSection?: string;
  /** Conta aberta: a seção Regras começa nela (US12, D1). */
  activeAccountId: string;
  /** Editor ou histórico das regras pedidos de fora ("Mais opções", "Ver"). */
  rulesRequest?: RulesRequest | null;
  onClose: () => void;
}

/** Configurações (US5, CA6): seções à esquerda, opções da seção à direita. Abrem em Geral (US7, CA4). */
export function Settings({ accounts, appearance, onAppearance, initialSection = "general", activeAccountId, rulesRequest, onClose }: SettingsProps) {
  const [section, setSection] = useState(initialSection);
  const current = SECTIONS.find((s) => s.id === section)!;
  return (
    <SettingsSheet sections={SECTIONS} current={section} onSection={setSection} onClose={onClose}>
      <SettingsTitle>{current.label}</SettingsTitle>
      {section === "general" ? (
        <GeneralSettings appearance={appearance} onChange={onAppearance} />
      ) : section === "email" ? (
        <>
          <OfflineSettings />
          <SyncSettings accounts={accounts} />
        </>
      ) : section === "notifications" ? (
        <NotificationSettings accounts={accounts} />
      ) : section === "privacy" ? (
        <PrivacySettings />
      ) : section === "rules" ? (
        <RulesSettings accounts={accounts} initialAccountId={activeAccountId} request={rulesRequest} />
      ) : section === "appearance" ? (
        <AppearanceSettings appearance={appearance} onChange={onAppearance} />
      ) : (
        <EmptyState title="Em breve" description={SOON[section]} />
      )}
    </SettingsSheet>
  );
}

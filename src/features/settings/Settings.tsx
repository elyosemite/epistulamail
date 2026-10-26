// Composição de DS: modais.html "Configurações" (.settings-sheet sobre as duas colunas, D2) + vazios.html (.empty
// "Em breve" nas seções ainda sem opções); criptografia.html: o gerenciador de chaves troca a folha (L66)
import { useState } from "react";
import type { Account, Appearance } from "../../lib/tauri";
import { CryptoSettings } from "../crypto/CryptoSettings";
import { KeyManager } from "../crypto/KeyManager";
import { EmptyState } from "../../ui/EmptyState";
import { ManagerSheet, SettingsSheet, SettingsTitle, type SettingsSection } from "../../ui/SettingsSheet";
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
  { id: "crypto", label: "Criptografia", icon: "lock" },
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
  /** Abre direto no gerenciador de chaves, numa chave ("Ver no gerenciador de chaves" da leitura, US13). */
  keysRequest?: { fingerprint: string | null } | null;
  onClose: () => void;
}

/** Configurações (US5, CA6): seções à esquerda, opções da seção à direita. Abrem em Geral (US7, CA4). */
export function Settings({ accounts, appearance, onAppearance, initialSection = "general", activeAccountId, rulesRequest, keysRequest, onClose }: SettingsProps) {
  const [section, setSection] = useState(keysRequest ? "crypto" : initialSection);
  const [manager, setManager] = useState<{ accountId: string; fingerprint: string | null } | null>(
    keysRequest ? { accountId: activeAccountId, fingerprint: keysRequest.fingerprint } : null,
  );
  const current = SECTIONS.find((s) => s.id === section)!;
  const managerAccount = manager && (accounts.find((a) => a.id === manager.accountId) ?? accounts[0]);
  if (managerAccount) {
    return (
      <ManagerSheet title="Chaves OpenPGP" onBack={() => setManager(null)} onClose={onClose}>
        <KeyManager account={managerAccount} initialKey={manager.fingerprint} />
      </ManagerSheet>
    );
  }
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
      ) : section === "crypto" ? (
        <CryptoSettings accounts={accounts} initialAccountId={activeAccountId} onManageKeys={(accountId) => setManager({ accountId, fingerprint: null })} />
      ) : section === "appearance" ? (
        <AppearanceSettings appearance={appearance} onChange={onAppearance} />
      ) : (
        <EmptyState title="Em breve" description={SOON[section]} />
      )}
    </SettingsSheet>
  );
}

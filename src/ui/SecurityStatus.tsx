// DS: criptografia.html "Ler mensagens criptografadas e assinadas" · .sec-status com .sec-badge (--info, --warning, --danger)
// (L67) e o painel "Segurança desta mensagem" (.menu como diálogo)
import { useState } from "react";
import { securityBadges } from "../lib/crypto";
import type { MessageSecurity } from "../lib/tauri";
import { Fingerprint } from "./Fingerprint";
import { Icon } from "./Icon";
import { MenuPanel, anchorBelow, type Anchor } from "./Menu";

const TECH = { openpgp: "OpenPGP", smime: "S/MIME" } as const;

const TRUST = { trusted: "Confiável (raiz do sistema)", expired: "Vencida", untrusted: "Não confiável: a raiz não está no sistema" } as const;

/** Selos de segurança abaixo do remetente; clicar abre os detalhes. */
export function SecurityStatus({ security, onManageKeys }: { security: MessageSecurity; onManageKeys?: () => void }) {
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  const badges = securityBadges(security);
  if (badges.length === 0) return null;
  const signature = security.signature;
  return (
    <>
      <div className="sec-status">
        {badges.map((b) => (
          <button
            key={b.text}
            type="button"
            className={["sec-badge", b.tone !== "success" && `sec-badge--${b.tone}`].filter(Boolean).join(" ")}
            aria-haspopup="dialog"
            onClick={(e) => setAnchor(anchorBelow(e.currentTarget))}
          >
            <Icon name={b.icon} />
            {b.text}
          </button>
        ))}
      </div>
      {anchor && (
        <MenuPanel label="Segurança desta mensagem" role="dialog" anchor={anchor} onClose={() => setAnchor(null)}>
          <div style={{ padding: "var(--space-200)", display: "grid", gap: "var(--space-150)", maxWidth: "24rem" }}>
            <h3 style={{ margin: 0, font: "var(--font-card)" }}>Segurança desta mensagem</h3>
            <dl style={{ display: "grid", gridTemplateColumns: "auto minmax(0, 1fr)", gap: "var(--space-050) var(--space-150)", margin: 0, font: "var(--font-body-small)" }}>
              <dt style={{ color: "var(--color-text-subtlest)" }}>Tecnologia</dt><dd style={{ margin: 0 }}>{TECH[security.technology]}</dd>
              <dt style={{ color: "var(--color-text-subtlest)" }}>Criptografia</dt>
              <dd style={{ margin: 0 }}>{security.encrypted ? (security.decrypted ? (security.postQuantum ? "Pós-quântica (ML-KEM-768 + X25519)" : "Sim") : "Sim, mas nenhuma das suas chaves abre") : "Não"}</dd>
              <dt style={{ color: "var(--color-text-subtlest)" }}>Assinatura</dt>
              <dd style={{ margin: 0 }}>
                {signature.state === "good"
                  ? "Confere"
                  : signature.state === "bad"
                    ? "Não confere: o conteúdo pode ter sido alterado"
                    : signature.state === "unknown_key"
                      ? security.technology === "smime" ? "O certificado de quem assinou não veio junto" : "A chave de quem assinou não está aqui"
                      : "Sem assinatura"}
              </dd>
              {signature.state === "good" && security.certificate && (
                <>
                  <dt style={{ color: "var(--color-text-subtlest)" }}>Certificado</dt>
                  <dd style={{ margin: 0 }}>{security.certificate.subject}{security.certificate.emailMatches ? "" : " (de outro e-mail)"}</dd>
                  <dt style={{ color: "var(--color-text-subtlest)" }}>Emissor</dt><dd style={{ margin: 0 }}>{security.certificate.issuer}</dd>
                  <dt style={{ color: "var(--color-text-subtlest)" }}>Cadeia</dt><dd style={{ margin: 0 }}>{TRUST[security.certificate.trust]}</dd>
                </>
              )}
              {signature.state === "good" && (
                <>
                  <dt style={{ color: "var(--color-text-subtlest)" }}>{security.technology === "smime" ? "SHA-256" : "Chave"}</dt><dd style={{ margin: 0 }}><Fingerprint value={signature.fingerprint} compact /></dd>
                </>
              )}
            </dl>
            {onManageKeys && (
              <button className="btn btn--subtle" type="button" style={{ justifySelf: "start" }} onClick={() => { setAnchor(null); onManageKeys(); }}>
                <Icon name="key" />
                {security.technology === "smime" ? "Ver no gerenciador de certificados" : "Ver no gerenciador de chaves"}
              </button>
            )}
          </div>
        </MenuPanel>
      )}
    </>
  );
}

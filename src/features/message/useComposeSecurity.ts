// Cadeado do composer (US13, CA6): configurações da conta, chaves dos destinatários e a busca na internet consentida (D5).
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { composeSecurity } from "../../lib/crypto";
import { parseRecipient } from "../../lib/recipients";
import { api, type CryptoSettings, type PgpKey, type RecipientKey } from "../../lib/tauri";

const STATUS_DELAY_MS = 300;

export function useComposeSecurity(accountId: string, recipients: string[], replyingToEncrypted: boolean) {
  const [settings, setSettings] = useState<CryptoSettings | null>(null);
  const [status, setStatus] = useState<RecipientKey[]>([]);
  const [choice, setChoice] = useState<{ encrypt?: boolean; sign?: boolean }>({});
  const [found, setFound] = useState<PgpKey[]>([]);
  const [searching, setSearching] = useState(false);
  const searched = useRef(new Set<string>());

  useEffect(() => {
    api.getCryptoSettings(accountId).then(setSettings).catch(() => setSettings(null));
  }, [accountId]);

  const addresses = useMemo(
    () => [...new Set(recipients.map(parseRecipient).filter((r) => r.valid).map((r) => r.address.toLowerCase()))].sort(),
    [recipients],
  );
  const addressKey = addresses.join(",");

  const refresh = useCallback(() => {
    if (addresses.length === 0) return setStatus([]);
    api.recipientsKeyStatus(addresses).then(setStatus).catch(() => undefined);
    // `addressKey` resume `addresses`.
  }, [addressKey]);

  useEffect(() => {
    const t = setTimeout(refresh, STATUS_DELAY_MS);
    return () => clearTimeout(t);
  }, [refresh]);

  const hasOwnKey = !!settings?.pgpKey;
  const security = composeSecurity(settings, hasOwnKey, recipients, status, choice, replyingToEncrypted);
  const keys = useMemo(() => (security.encrypt ? new Map(status.map((s) => [s.address, s.hasKey])) : null), [security.encrypt, status]);

  /** Busca as chaves que faltam (WKD e keys.openpgp.org). Chamar só com o consentimento. */
  const search = useCallback(async (missing: string[]) => {
    setSearching(true);
    try {
      const results = await Promise.all(
        missing.map((a) => {
          searched.current.add(a);
          return api.searchPgpKey(a).catch(() => [] as PgpKey[]);
        }),
      );
      setFound((before) => [...before.filter((k) => !missing.some((a) => k.emails.includes(a))), ...results.flat()]);
      refresh();
      return results.flat();
    } finally {
      setSearching(false);
    }
  }, [refresh]);

  // "Sempre": busca sozinha, uma vez por endereço.
  const missingKey = security.missing.join(",");
  useEffect(() => {
    if (settings?.searchOnline !== "always") return;
    const pending = security.missing.filter((a) => !searched.current.has(a));
    if (pending.length > 0) void search(pending);
  }, [settings?.searchOnline, missingKey, search]);

  const accept = useCallback(async (fingerprints: string[]) => {
    await Promise.all(fingerprints.map((fp) => api.setPgpAcceptance(fp, "unverified")));
    refresh();
  }, [refresh]);

  /** Chaves achadas na busca para quem ainda está sem chave e que o usuário ainda não decidiu. */
  const foundForMissing = found.filter((k) => k.acceptance === "undecided" && security.missing.some((a) => k.emails.includes(a)));

  return {
    enabled: hasOwnKey,
    settings,
    security,
    keys,
    setChoice,
    search,
    searching,
    searchedAll: security.missing.every((a) => searched.current.has(a)),
    found: foundForMissing,
    accept,
  };
}

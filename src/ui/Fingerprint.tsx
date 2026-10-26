// DS: criptografia.html · .fingerprint (--compact) (L65): impressão digital em grupos de 4, mono, selecionável
import { fingerprintGroups, fingerprintShort } from "../lib/crypto";

export function Fingerprint({ value, compact }: { value: string; compact?: boolean }) {
  const groups = compact ? fingerprintShort(value) : fingerprintGroups(value);
  return (
    <span className={compact ? "fingerprint fingerprint--compact" : "fingerprint"} aria-label={`Impressão digital ${fingerprintGroups(value).join(" ")}`}>
      {groups.map((g, i) => <span key={i}>{g}</span>)}
    </span>
  );
}

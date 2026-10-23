// DS: carregamento.html · .spinner, .spinner--sm, .spinner--lg (dentro de .btn o DS reduz para 14px)
type SpinnerSize = "sm" | "md" | "lg";

export function Spinner({ size = "md", label, className }: { size?: SpinnerSize; label?: string; className?: string }) {
  const cls = ["spinner", size !== "md" && `spinner--${size}`, className].filter(Boolean).join(" ");
  if (!label) return <span className={cls} aria-hidden="true" />;
  return <span className={cls} role="status" aria-label={label} />;
}

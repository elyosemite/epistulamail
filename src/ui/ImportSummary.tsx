// DS: arquivos.html · .import-summary (L72): entraram, repetidas e falharam, lado a lado (US15)
export function ImportSummary({ imported, duplicates, failed }: { imported: number; duplicates: number; failed: number }) {
  const n = (v: number) => v.toLocaleString("pt-BR");
  return (
    <dl className="import-summary">
      <div><dt>Entraram</dt><dd>{n(imported)}</dd></div>
      <div><dt>Repetidas</dt><dd>{n(duplicates)}</dd></div>
      <div className={failed > 0 ? "is-danger" : undefined}><dt>Falharam</dt><dd>{n(failed)}</dd></div>
    </dl>
  );
}

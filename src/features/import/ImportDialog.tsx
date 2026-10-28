// Composição de DS: arquivos.html (assistente de três passos, resumo L72, avatar L71) · modais.html (.modal--medium,
// .modal__foot--split), configuracoes (.settings__group, .setting, .setting-list com __head, __row, .is-checked,
// __name, __title, __sub, __end, __size), carregamento.html (.meter, .progress, --warning), mensagens.html
// (.section-message), etiquetas (.lozenge), campos.html (.field), .wizard-steps (criptografia.html)
import { useEffect, useMemo, useState } from "react";
import { folderIcon } from "../../lib/folders";
import { formatBytes } from "../../lib/format";
import {
  FORMAT_LABEL,
  WHERE_TO_FIND,
  defaultSelection,
  diskLine,
  diskShare,
  failedTotal,
  failureLines,
  fitsOnDisk,
  folderRows,
  messagesLabel,
  missingBytes,
  progressLine,
  progressShare,
  selectedCount,
  suggestedName,
} from "../../lib/importing";
import { api, isAppError, onBackendEvent, type Account, type AppError, type Archive, type ImportProgress, type ImportScan, type ImportStop, type SourceFolder } from "../../lib/tauri";
import { AccountAvatar } from "../../ui/AccountAvatar";
import { Button } from "../../ui/Button";
import { TextField } from "../../ui/Field";
import { Icon } from "../../ui/Icon";
import { ImportSummary } from "../../ui/ImportSummary";
import { Lozenge } from "../../ui/Lozenge";
import { Modal } from "../../ui/Modal";
import { SectionMessage } from "../../ui/SectionMessage";
import { Spinner } from "../../ui/Spinner";

type Step =
  | { kind: "pick"; error?: AppError }
  | { kind: "scanning"; path: string }
  | { kind: "review"; path: string; scan: ImportScan }
  | { kind: "importing"; account: Account; progress: ImportProgress | null }
  | { kind: "done"; archive: Archive; name: string; stopped: ImportStop };

interface ImportDialogProps {
  /** Caminho já escolhido (arquivo solto sobre a janela): começa no passo 2. */
  initialPath?: string | null;
  /** "Importar mais neste arquivo": as mensagens entram num arquivo local que já existe. */
  into?: Account | null;
  onClose: () => void;
  /** O usuário fechou com a importação ainda correndo; o progresso segue na coluna de pastas. */
  onBackground: (account: Account) => void;
  onOpen: (accountId: string) => void;
}

const UNKNOWN: AppError = { kind: "ImportUnrecognized", message: "Algo deu errado ao ler o arquivo." };

/** Abrir um arquivo de e-mail (CA1, CA2): escolher, revisar, importar e o resumo. */
export function ImportDialog({ initialPath, into, onClose, onBackground, onOpen }: ImportDialogProps) {
  const [step, setStep] = useState<Step>(initialPath ? { kind: "scanning", path: initialPath } : { kind: "pick" });

  const scanning = step.kind === "scanning" ? step.path : null;
  useEffect(() => {
    if (!scanning) return;
    let live = true;
    api
      .scanMailSource(scanning)
      .then((scan) => live && setStep({ kind: "review", path: scanning, scan }))
      .catch((error) => live && setStep({ kind: "pick", error: isAppError(error) ? error : UNKNOWN }));
    return () => {
      live = false;
    };
  }, [scanning]);

  const importingId = step.kind === "importing" ? step.account.id : null;
  useEffect(() => {
    if (!importingId) return;
    const offs = [
      onBackendEvent("import-progress", (p) => {
        if (p.accountId === importingId) setStep((s) => (s.kind === "importing" ? { ...s, progress: p } : s));
      }),
      onBackendEvent("import-finished", ({ archive, stopped }) => {
        if (archive.accountId !== importingId) return;
        setStep((s) => (s.kind === "importing" ? { kind: "done", archive, name: s.account.displayName, stopped } : s));
      }),
    ];
    return () => offs.forEach((off) => off());
  }, [importingId]);

  const pick = async (folder: boolean) => {
    try {
      const path = await api.pickMailSource(folder);
      if (path) setStep({ kind: "scanning", path });
    } catch (error) {
      setStep({ kind: "pick", error: isAppError(error) ? error : UNKNOWN });
    }
  };

  const close = () => {
    if (step.kind === "scanning") void api.cancelMailScan().catch(() => undefined);
    if (step.kind === "importing") onBackground(step.account);
    onClose();
  };

  switch (step.kind) {
    case "pick":
      return <PickStep error={step.error} onPick={pick} onClose={close} />;
    case "scanning":
      return (
        <Modal title="Abrir arquivo de e-mail" medium onClose={close} footer={<Button appearance="subtle" onClick={close}>Cancelar</Button>}>
          <div style={{ display: "grid", justifyItems: "center", gap: "var(--space-150)", padding: "var(--space-300) 0" }}>
            <Spinner size="lg" label="Lendo o arquivo" />
            <p style={{ margin: 0, color: "var(--color-text-subtle)" }}>Lendo o arquivo para contar as mensagens. Arquivos grandes levam alguns segundos por GB.</p>
          </div>
        </Modal>
      );
    case "review":
      return (
        <ReviewStep
          scan={step.scan}
          into={into ?? null}
          onBack={() => setStep({ kind: "pick" })}
          onClose={close}
          onStart={async (name, selected) => {
            try {
              const account = await api.startMailImport(step.path, name, selected, into?.id ?? null);
              setStep({ kind: "importing", account, progress: null });
            } catch (error) {
              throw isAppError(error) ? error : UNKNOWN;
            }
          }}
        />
      );
    case "importing": {
      const p = step.progress;
      const imported = p?.imported ?? 0;
      const estimate = p?.estimate ?? 0;
      return (
        <Modal
          title={`Importando ${step.account.displayName}`}
          onClose={close}
          footer={
            <>
              <Button appearance="subtle" onClick={() => void api.cancelMailImport(step.account.id).catch(() => undefined)}>Cancelar</Button>
              <Button onClick={close}>Continuar em segundo plano</Button>
            </>
          }
        >
          <div style={{ display: "grid", gap: "var(--space-200)" }}>
            <p className="wizard-steps" style={{ margin: 0 }}>Passo 3 de 3</p>
            <div className="meter" style={{ padding: 0 }} role="progressbar" aria-label="Mensagens importadas" aria-valuemin={0} aria-valuemax={estimate || undefined} aria-valuenow={imported}>
              <div className="progress"><span style={{ width: `${progressShare(imported, estimate)}%` }} /></div>
              {p ? progressLine(imported, estimate, p.folder) : "Começando…"}
            </div>
            <p style={{ margin: 0, color: "var(--color-text-subtle)" }}>Você já pode ler o que entrou. Cancelar mantém as mensagens importadas até aqui.</p>
          </div>
        </Modal>
      );
    }
    case "done":
      return <DoneStep archive={step.archive} name={step.name} stopped={step.stopped} onClose={onClose} onOpen={() => { onClose(); onOpen(step.archive.accountId); }} />;
  }
}

function PickStep({ error, onPick, onClose }: { error?: AppError; onPick: (folder: boolean) => void; onClose: () => void }) {
  return (
    <Modal title="Abrir arquivo de e-mail" medium onClose={onClose} footer={<Button appearance="subtle" onClick={onClose}>Cancelar</Button>}>
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <p className="wizard-steps" style={{ margin: 0 }}>Passo 1 de 3</p>
        {error && (
          <SectionMessage appearance="error" title={error.kind === "ImportUnrecognized" ? "Este arquivo não parece ser de e-mail" : "Não foi possível abrir"}>
            <p>{error.kind === "ImportUnrecognized" ? "O EpistulaMail abre .mbox, .pst, .msg, .eml, .emlx e pastas Maildir. Se é um backup compactado (.zip do Takeout), descompacte antes." : error.message}</p>
          </SectionMessage>
        )}
        <p style={{ margin: 0, color: "var(--color-text-subtle)" }}>O arquivo é lido neste computador e copiado para o banco cifrado do EpistulaMail. Nada vai para a internet, e o original não é alterado.</p>
        <div style={{ display: "flex", flexWrap: "wrap", gap: "var(--space-100)" }}>
          <Button appearance="primary" icon="file" onClick={() => onPick(false)}>Escolher arquivo…</Button>
          <Button icon="folder" onClick={() => onPick(true)}>Escolher pasta…</Button>
        </div>
        <div className="settings__group" style={{ margin: 0 }}>
          <h3 className="settings__group-title">Onde encontro o meu backup?</h3>
          <div style={{ display: "grid", gap: "var(--space-150)" }}>
            {WHERE_TO_FIND.map((w) => (
              <div key={w.title} className="setting" style={{ gridTemplateColumns: "auto minmax(0, 1fr)" }}>
                <span className="avatar avatar--archive" aria-hidden="true"><Icon name={w.icon} /></span>
                <div>
                  <div className="setting__label">{w.title}</div>
                  <p className="setting__desc">{w.desc}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </Modal>
  );
}

interface ReviewProps {
  scan: ImportScan;
  into: Account | null;
  onBack: () => void;
  onClose: () => void;
  onStart: (name: string, selected: string[]) => Promise<void>;
}

function ReviewStep({ scan, into, onBack, onClose, onStart }: ReviewProps) {
  const [selected, setSelected] = useState(() => defaultSelection(scan));
  const [name, setName] = useState(into?.displayName ?? suggestedName(scan.name));
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const { folders, labels } = useMemo(() => folderRows(scan.folders), [scan.folders]);
  const count = selectedCount(scan, selected);
  const fits = fitsOnDisk(scan, selected);
  const total = scan.folders.length;

  const toggle = (keys: string[], on: boolean) =>
    setSelected((current) => {
      const next = new Set(current);
      keys.forEach((k) => (on ? next.add(k) : next.delete(k)));
      return next;
    });
  const labelsOn = labels.length > 0 && labels.every((l) => selected.has(l.key));

  const start = async () => {
    setStarting(true);
    setError(null);
    try {
      await onStart(name.trim(), Array.from(selected));
    } catch (e) {
      setError(isAppError(e) ? e : UNKNOWN);
      setStarting(false);
    }
  };

  return (
    <Modal
      title={into ? `Importar mais em ${into.displayName}` : "Abrir arquivo de e-mail"}
      medium
      splitFooter
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" onClick={onBack}>Voltar</Button>
          <Button appearance="subtle" onClick={onClose}>Cancelar</Button>
          <Button appearance="primary" loading={starting} disabled={count === 0 || fits === false || (!into && !name.trim())} onClick={() => void start()}>
            {`Importar ${messagesLabel(count)}`}
          </Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        <p className="wizard-steps" style={{ margin: 0 }}>Passo 2 de 3</p>
        <div className="setting-list">
          <div className="setting-list__row">
            <AccountAvatar initial="" colorClass={null} photo={null} size="lg" archive />
            <div className="setting-list__name">
              <span className="setting-list__title">{scan.name}</span>
              <span className="setting-list__sub"><Lozenge appearance="success">{FORMAT_LABEL[scan.format]}</Lozenge></span>
            </div>
            <span className="setting-list__end"><span className="setting-list__size">{formatBytes(scan.bytes)}</span></span>
          </div>
        </div>
        {!into && (
          <TextField label="Nome no EpistulaMail" value={name} maxLength={32} onChange={(e) => setName(e.target.value)}>
            <span className="field__help">Aparece na coluna de contas. Dá para trocar depois.</span>
          </TextField>
        )}
        {total > 1 && (
          <div className="setting-list">
            <div className="setting-list__head">
              <input
                type="checkbox"
                className="checkbox"
                aria-label="Marcar todas as pastas"
                checked={selected.size === total}
                ref={(el) => { if (el) el.indeterminate = selected.size > 0 && selected.size < total; }}
                onChange={(e) => toggle(scan.folders.map((f) => f.key), e.target.checked)}
              />
              <span>{`${selected.size} de ${total} pastas · ${messagesLabel(count)}`}</span>
            </div>
            {folders.map((f) => (
              <FolderRow key={f.key} folder={f} checked={selected.has(f.key)} onChange={(on) => toggle([f.key], on)} />
            ))}
            {labels.length > 0 && (
              <div className={["setting-list__row", labelsOn && "is-checked"].filter(Boolean).join(" ")}>
                <input type="checkbox" className="checkbox" aria-label="Marcadores" checked={labelsOn} onChange={(e) => toggle(labels.map((l) => l.key), e.target.checked)} />
                <Icon name="tag" />
                <div className="setting-list__name">
                  <span className="setting-list__title">{`Marcadores (${labels.length})`}</span>
                  <span className="setting-list__sub">{labelsSummary(labels)}</span>
                </div>
                <span className="setting-list__end"><span className="setting-list__size">—</span></span>
              </div>
            )}
          </div>
        )}
        <div className="meter" style={{ padding: 0 }}>
          <div className={fits === false ? "progress progress--warning" : "progress"}><span style={{ width: `${fits === false ? 100 : diskShare(scan, selected)}%` }} /></div>
          {diskLine(scan, selected)}
        </div>
        {fits === false && (
          <SectionMessage appearance="error" title="Não cabe neste disco">
            <p>{`Faltam cerca de ${formatBytes(missingBytes(scan, selected))}, contando a margem que o sistema precisa. Desmarque pastas ou libere espaço.`}</p>
          </SectionMessage>
        )}
        {error && <SectionMessage appearance="error" title="Não foi possível começar"><p>{error.message}</p></SectionMessage>}
        <SectionMessage appearance="warning" title="O original continua sem cifra">
          <p>O EpistulaMail guarda uma cópia cifrada e não mexe no arquivo. Ele continua legível onde está; apague-o você mesmo quando não precisar mais dele.</p>
        </SectionMessage>
      </div>
    </Modal>
  );
}

function labelsSummary(labels: SourceFolder[]): string {
  const names = labels.map((l) => l.name);
  return names.length <= 2 ? names.join(" e ") : `${names.slice(0, 2).join(", ")} e mais ${names.length - 2}`;
}

function FolderRow({ folder, checked, onChange }: { folder: SourceFolder; checked: boolean; onChange: (on: boolean) => void }) {
  const leaf = folder.depth > 0 ? folder.name.split("/").slice(-1)[0] : folder.name;
  return (
    <div className={["setting-list__row", checked && "is-checked"].filter(Boolean).join(" ")}>
      <input type="checkbox" className="checkbox" aria-label={folder.name} checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span style={{ paddingInlineStart: `calc(var(--space-200) * ${folder.depth})`, display: "inline-flex" }}><Icon name={folderIcon(folder.role)} /></span>
      <div className="setting-list__name">
        <span className="setting-list__title">{leaf}</span>
        <span className="setting-list__sub">{messagesLabel(folder.count)}</span>
      </div>
      <span className="setting-list__end"><span className="setting-list__size">{folder.bytes > 0 ? formatBytes(folder.bytes) : "—"}</span></span>
    </div>
  );
}

function DoneStep({ archive, name, stopped, onClose, onOpen }: { archive: Archive; name: string; stopped: ImportStop; onClose: () => void; onOpen: () => void }) {
  const failed = failedTotal(archive.failures);
  const title = stopped === null ? `${name} está pronto` : stopped === "cancelled" ? `Importação de ${name} cancelada` : `A importação de ${name} parou`;
  return (
    <Modal
      title={title}
      onClose={onClose}
      footer={
        <>
          {stopped !== null && <Button appearance="subtle" onClick={onClose}>Fechar</Button>}
          <Button appearance="primary" onClick={onOpen}>{`Abrir ${name}`}</Button>
        </>
      }
    >
      <div style={{ display: "grid", gap: "var(--space-200)" }}>
        {stopped === "no_space" && <SectionMessage appearance="warning" title="O disco ficou sem espaço"><p>Libere espaço e continue pelo menu do arquivo. As que já entraram ficam.</p></SectionMessage>}
        {stopped === "source_missing" && <SectionMessage appearance="warning" title="O arquivo de origem sumiu"><p>Ele não está mais onde estava. As que já entraram ficam.</p></SectionMessage>}
        {stopped === "error" && <SectionMessage appearance="error" title="Algo deu errado ao gravar"><p>As que já entraram ficam. Tente continuar pelo menu do arquivo.</p></SectionMessage>}
        <ImportSummary imported={archive.imported} duplicates={archive.duplicates} failed={failed} />
        {failureLines(archive.failures).length > 0 && (
          <div style={{ display: "grid", gap: "var(--space-150)" }}>
            {failureLines(archive.failures).map((f) => (
              <div key={f.title} className="setting">
                <div>
                  <div className="setting__label">{f.title}</div>
                  <p className="setting__desc">{f.desc}</p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </Modal>
  );
}

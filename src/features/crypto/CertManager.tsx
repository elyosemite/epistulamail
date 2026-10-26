// Composição de DS: criptografia.html "S/MIME: pedir certificado e gerenciar" · .key-manager (__bar, __body), .segmented
// (Meus, Pessoas, Pedidos pendentes), .key-table (__who, linha com aria-selected), .key-details (__title) e .fingerprint (L65);
// .pem-box (L69) do certificado e do pedido; modais.html .modal--warning para excluir
import { useCallback, useEffect, useMemo, useState, type KeyboardEvent } from "react";
import { CERT_ORIGIN, certStatus, certUsage, keyDate, TRUST_LABEL } from "../../lib/crypto";
import { keyFileName, saveText } from "../../lib/files";
import { api, type Account, type SmimeCert, type SmimeCertKind, type SmimeRequest } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { Fingerprint } from "../../ui/Fingerprint";
import { Lozenge } from "../../ui/Lozenge";
import { Modal } from "../../ui/Modal";
import { PemBox } from "../../ui/PemBox";
import { Segmented } from "../../ui/Segmented";
import { Spinner } from "../../ui/Spinner";
import { useToast } from "../../ui/Toast";
import { ImportCertDialog, Pkcs12Dialog, RequestCertDialog, type RequestDefaults } from "./SmimeDialogs";

type Tab = "mine" | "people" | "requests";

type Dialog =
  | { kind: "request"; defaults?: RequestDefaults }
  | { kind: "import" }
  | { kind: "public"; cert: SmimeCert; pem: string }
  | { kind: "p12"; cert: SmimeCert }
  | { kind: "delete"; cert: SmimeCert }
  | { kind: "csr"; request: SmimeRequest }
  | { kind: "delete-request"; request: SmimeRequest }
  | null;

const KIND_OF: Record<string, SmimeCertKind> = { "RSA 3072": "rsa3072", "ECDSA P-256": "p256" };

/** Gerenciador de certificados S/MIME (CA2 da US14): os meus, os de pessoas e os pedidos que esperam a CA. */
export function CertManager({ account, initialCert }: { account: Account; initialCert?: string | null }) {
  const [certs, setCerts] = useState<SmimeCert[] | null>(null);
  const [requests, setRequests] = useState<SmimeRequest[]>([]);
  const [tab, setTab] = useState<Tab>("mine");
  const [selected, setSelected] = useState<string | null>(initialCert ?? null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const toast = useToast();

  const load = useCallback(() => {
    api.listSmimeCerts().then(setCerts).catch(() => { setCerts([]); toast({ kind: "error", title: "Não foi possível carregar os certificados" }); });
    api.listSmimeRequests().then(setRequests).catch(() => setRequests([]));
  }, [toast]);
  useEffect(load, [load]);

  // Aberto numa assinatura recebida: começa na aba Pessoas.
  useEffect(() => {
    if (initialCert && certs?.some((c) => c.fingerprint === initialCert && !c.mine)) setTab("people");
  }, [initialCert, certs]);

  const shown = useMemo(() => (certs ?? []).filter((c) => (tab === "mine" ? c.mine : !c.mine)), [certs, tab]);
  const current = tab === "requests" ? null : shown.find((c) => c.fingerprint === selected) ?? shown[0] ?? null;

  const exportPublic = async (cert: SmimeCert) => {
    try {
      setDialog({ kind: "public", cert, pem: await api.exportSmimeCert(cert.fingerprint) });
    } catch {
      toast({ kind: "error", title: "Não foi possível exportar o certificado" });
    }
  };

  const renew = (cert: SmimeCert) =>
    setDialog({ kind: "request", defaults: { name: cert.subject, organization: cert.organization, country: "BR", kind: KIND_OF[cert.algorithm] ?? "rsa3072" } });

  const onTableKey = (e: KeyboardEvent<HTMLTableSectionElement>) => {
    const step = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (!step || shown.length === 0) return;
    e.preventDefault();
    const i = current ? shown.indexOf(current) : -1;
    const next = shown[Math.min(shown.length - 1, Math.max(0, i + step))];
    setSelected(next.fingerprint);
    e.currentTarget.querySelector<HTMLElement>(`[data-fp="${next.fingerprint}"]`)?.focus();
  };

  if (!certs) return <Spinner label="Carregando os certificados" />;

  return (
    <div className="key-manager" style={{ padding: "var(--space-200)", overflow: "hidden", flex: 1 }}>
      <div className="key-manager__bar">
        <Segmented<Tab>
          label="Certificados"
          value={tab}
          onChange={setTab}
          options={[
            { value: "mine", label: "Meus" },
            { value: "people", label: "Pessoas" },
            { value: "requests", label: requests.length > 0 ? `Pedidos pendentes · ${requests.length}` : "Pedidos pendentes" },
          ]}
        />
        <span style={{ flex: 1 }} />
        <Button appearance="subtle" icon="upload" onClick={() => setDialog({ kind: "import" })}>Importar…</Button>
        <Button appearance="primary" icon="certificate" onClick={() => setDialog({ kind: "request" })}>Pedir certificado…</Button>
      </div>

      {tab === "requests" ? (
        <div style={{ overflow: "auto" }}>
          {requests.length === 0 ? (
            <p className="setting__desc">Nenhum pedido esperando certificado.</p>
          ) : (
            <table className="key-table">
              <thead><tr><th>Para</th><th>Feito em</th><th>Algoritmo</th><th /></tr></thead>
              <tbody>
                {requests.map((r) => (
                  <tr key={r.id}>
                    <td><span className="key-table__who"><b>{r.name}</b>{r.email}</span></td>
                    <td>{keyDate(r.createdAt)}</td>
                    <td>{r.algorithm}</td>
                    <td>
                      <div className="row" style={{ gap: "var(--space-075)", justifyContent: "flex-end", flexWrap: "wrap" }}>
                        <Button appearance="subtle" compact icon="copy" onClick={() => setDialog({ kind: "csr", request: r })}>Ver pedido</Button>
                        <Button appearance="subtle" compact icon="upload" onClick={() => setDialog({ kind: "import" })}>Importar certificado emitido…</Button>
                        <Button appearance="subtle" compact icon="trash" onClick={() => setDialog({ kind: "delete-request", request: r })}>Excluir</Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      ) : (
        <div className="key-manager__body">
          <div style={{ overflow: "auto" }}>
            {shown.length === 0 ? (
              <p className="setting__desc">
                {tab === "mine"
                  ? "Nenhum certificado seu ainda. Peça um a uma autoridade certificadora ou importe um .p12."
                  : "Os certificados de quem manda mensagens assinadas com S/MIME aparecem aqui sozinhos."}
              </p>
            ) : (
              <table className="key-table">
                <thead><tr><th>Titular</th><th>Emissor</th><th>Validade</th><th>Situação</th></tr></thead>
                <tbody onKeyDown={onTableKey}>
                  {shown.map((c) => {
                    const status = certStatus(c);
                    const isCurrent = c === current;
                    return (
                      <tr key={c.fingerprint} data-fp={c.fingerprint} aria-selected={isCurrent} tabIndex={isCurrent ? 0 : -1} style={{ cursor: "default" }} onClick={() => setSelected(c.fingerprint)}>
                        <td><span className="key-table__who"><b>{c.subject}</b>{c.emails[0]}</span></td>
                        <td>{c.issuer}</td>
                        <td>{keyDate(c.notAfter)}</td>
                        <td><Lozenge appearance={status.tone}>{status.text}</Lozenge></td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
          </div>
          {current && (
            <aside className="key-details" aria-labelledby={`cd-${current.fingerprint}`}>
              <h3 className="key-details__title" id={`cd-${current.fingerprint}`}>{current.subject}{current.mine && <> <Lozenge appearance="success">Meu</Lozenge></>}</h3>
              <dl>
                <dt>E-mail</dt><dd>{current.emails.join(", ") || "Nenhum"}</dd>
                {current.organization && (<><dt>Organização</dt><dd>{current.organization}</dd></>)}
                <dt>Emissor</dt><dd>{current.issuer}{current.chain.length > 0 ? ` · ${current.chain.join(" → ")}` : ""}</dd>
                <dt>Cadeia</dt><dd>{TRUST_LABEL[current.trust]}</dd>
                <dt>Uso</dt><dd>{certUsage(current)}</dd>
                <dt>Algoritmo</dt><dd>{current.algorithm}</dd>
                <dt>Vale de</dt><dd>{keyDate(current.notBefore)} a {keyDate(current.notAfter)}</dd>
                <dt>Chegou por</dt><dd>{CERT_ORIGIN[current.origin]}</dd>
              </dl>
              <div className="field"><span className="field__label">Impressão digital SHA-256</span><Fingerprint value={current.fingerprint} compact /></div>
              <div className="row" style={{ gap: "var(--space-075)", flexWrap: "wrap" }}>
                <Button appearance="subtle" compact icon="download" onClick={() => void exportPublic(current)}>Exportar .cer</Button>
                {current.mine && <Button appearance="subtle" compact icon="shield" onClick={() => setDialog({ kind: "p12", cert: current })}>Backup .p12…</Button>}
                {current.mine && <Button appearance="subtle" compact icon="refresh" onClick={() => renew(current)}>Renovar…</Button>}
                <Button appearance="subtle" compact icon="trash" onClick={() => setDialog({ kind: "delete", cert: current })}>Excluir</Button>
              </div>
            </aside>
          )}
        </div>
      )}

      {dialog?.kind === "request" && (
        <RequestCertDialog
          account={account}
          defaults={dialog.defaults}
          onClose={() => setDialog(null)}
          onCreated={() => { load(); setTab("requests"); }}
          onImport={() => setDialog({ kind: "import" })}
        />
      )}
      {dialog?.kind === "import" && (
        <ImportCertDialog
          onClose={() => setDialog(null)}
          onImported={(list) => {
            setDialog(null);
            load();
            const first = list[0];
            if (first) {
              setTab(first.mine ? "mine" : "people");
              setSelected(first.fingerprint);
            }
            toast({ kind: "success", title: list.length === 1 ? "Certificado importado" : `${list.length} certificados importados` });
          }}
        />
      )}
      {dialog?.kind === "public" && (
        <Modal
          title="Certificado público"
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button appearance="subtle" onClick={() => setDialog(null)}>Fechar</Button>
              <Button appearance="primary" icon="download" onClick={() => { saveText(keyFileName(dialog.cert.subject, "certificado", "cer"), dialog.pem, "application/pkix-cert"); setDialog(null); }}>Salvar .cer</Button>
            </>
          }
        >
          <div style={{ display: "grid", gap: "var(--space-150)" }}>
            <p className="setting__desc" style={{ margin: 0 }}>Pode mandar para qualquer pessoa: com ele, escrevem para {dialog.cert.mine ? "você" : dialog.cert.subject} com criptografia. A chave privada não vai junto.</p>
            <PemBox text={dialog.pem} label={`Certificado de ${dialog.cert.subject}`} />
          </div>
        </Modal>
      )}
      {dialog?.kind === "csr" && (
        <Modal
          title="Pedido de certificado"
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button appearance="subtle" onClick={() => setDialog(null)}>Fechar</Button>
              <Button appearance="primary" icon="download" onClick={() => saveText(keyFileName(dialog.request.name, "pedido", "csr"), dialog.request.csrPem, "application/pkcs10")}>Salvar .csr…</Button>
            </>
          }
        >
          <PemBox text={dialog.request.csrPem} label="Pedido de certificado" />
        </Modal>
      )}
      {dialog?.kind === "p12" && <Pkcs12Dialog cert={dialog.cert} onClose={() => setDialog(null)} />}
      {dialog?.kind === "delete" && (
        <Modal
          title={dialog.cert.mine ? "Excluir o seu certificado?" : "Excluir o certificado?"}
          appearance="warning"
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button appearance="subtle" onClick={() => setDialog(null)}>Cancelar</Button>
              <Button
                appearance="warning"
                onClick={() => {
                  const fp = dialog.cert.fingerprint;
                  setDialog(null);
                  api.deleteSmimeCert(fp).then(() => { setSelected(null); load(); toast({ kind: "success", title: "Certificado excluído" }); }).catch(() => toast({ kind: "error", title: "Não foi possível excluir" }));
                }}
              >
                Excluir
              </Button>
            </>
          }
        >
          {dialog.cert.mine ? (
            <p>Sem a chave dele, as mensagens criptografadas para você não abrem mais neste computador. Só exclua se tiver um backup .p12. As contas que o usam ficam sem S/MIME.</p>
          ) : (
            <p>As mensagens para {dialog.cert.emails[0] ?? dialog.cert.subject} deixam de poder ir criptografadas com S/MIME até o certificado voltar.</p>
          )}
        </Modal>
      )}
      {dialog?.kind === "delete-request" && (
        <Modal
          title="Excluir o pedido?"
          appearance="warning"
          onClose={() => setDialog(null)}
          footer={
            <>
              <Button appearance="subtle" onClick={() => setDialog(null)}>Cancelar</Button>
              <Button
                appearance="warning"
                onClick={() => {
                  const id = dialog.request.id;
                  setDialog(null);
                  api.deleteSmimeRequest(id).then(load).catch(() => toast({ kind: "error", title: "Não foi possível excluir" }));
                }}
              >
                Excluir
              </Button>
            </>
          }
        >
          <p>A chave gerada para este pedido é apagada. Se a autoridade ainda emitir o certificado, ele não vai servir: será preciso pedir de novo.</p>
        </Modal>
      )}
    </div>
  );
}

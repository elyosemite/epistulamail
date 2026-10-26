-- US14: certificados S/MIME e pedidos de certificado.

-- Meus certificados (com a chave privada) e os dos contatos. A chave privada fica sem senha própria,
-- porque o banco já é cifrado (e, com a senha do Epistula, fecha ao bloquear).
CREATE TABLE smime_cert (
    fingerprint   TEXT PRIMARY KEY,            -- SHA-256 do certificado
    der           BLOB NOT NULL,
    -- Intermediários até a raiz (PEM), como vieram no PKCS#12, no .p7b ou na assinatura.
    chain_pem     TEXT NOT NULL DEFAULT '',
    private_key   BLOB,                        -- PKCS#8, só nos meus
    subject       TEXT NOT NULL,
    organization  TEXT,
    issuer        TEXT NOT NULL,
    serial        TEXT NOT NULL,
    algorithm     TEXT NOT NULL,
    not_before    INTEGER NOT NULL,
    not_after     INTEGER NOT NULL,
    can_sign      INTEGER NOT NULL,
    can_encrypt   INTEGER NOT NULL,
    self_signed   INTEGER NOT NULL DEFAULT 0,
    -- Pedido feito aqui, arquivo de certificado, PKCS#12 ou assinatura de uma mensagem recebida.
    origin        TEXT NOT NULL CHECK (origin IN ('request', 'imported', 'pkcs12', 'message')),
    added_at      INTEGER NOT NULL
);

CREATE TABLE smime_cert_email (
    email        TEXT NOT NULL,
    fingerprint  TEXT NOT NULL REFERENCES smime_cert (fingerprint) ON DELETE CASCADE,
    PRIMARY KEY (email, fingerprint)
);

-- Pedidos aguardando o certificado da CA (CA1): o par de chaves gerado aqui e o CSR.
CREATE TABLE smime_request (
    id            INTEGER PRIMARY KEY,
    account_id    TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    email         TEXT NOT NULL,
    name          TEXT NOT NULL,
    organization  TEXT,
    country       TEXT,
    algorithm     TEXT NOT NULL,
    private_key   BLOB NOT NULL,
    public_key    BLOB NOT NULL,
    csr_pem       TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

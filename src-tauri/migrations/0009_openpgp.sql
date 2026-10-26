-- US13: chaves OpenPGP e configurações de criptografia por conta.

-- Um chaveiro só para o app (como no Thunderbird): minhas chaves e as dos contatos. A parte secreta
-- fica sem senha própria, porque o banco já é cifrado (e, com a senha do Epistula, fecha ao bloquear).
CREATE TABLE pgp_key (
    fingerprint   TEXT PRIMARY KEY,
    public_key    BLOB NOT NULL,
    secret_key    BLOB,
    user_ids      TEXT NOT NULL,
    algorithm     TEXT NOT NULL,
    version       INTEGER NOT NULL,
    post_quantum  INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL,
    expires_at    INTEGER,
    revoked       INTEGER NOT NULL DEFAULT 0,
    -- Aceitação da chave de um contato (CA3): ainda não, rejeitada, aceita sem conferir, conferida.
    acceptance    TEXT NOT NULL DEFAULT 'undecided' CHECK (acceptance IN ('undecided', 'rejected', 'unverified', 'verified')),
    -- De onde veio: gerada aqui, importada, Autocrypt, anexo de mensagem ou busca na internet.
    origin        TEXT NOT NULL CHECK (origin IN ('generated', 'imported', 'autocrypt', 'attachment', 'search')),
    added_at      INTEGER NOT NULL
);

-- E-mails de cada chave (das identidades), para achar a chave de um destinatário.
CREATE TABLE pgp_key_email (
    email        TEXT NOT NULL,
    fingerprint  TEXT NOT NULL REFERENCES pgp_key (fingerprint) ON DELETE CASCADE,
    PRIMARY KEY (email, fingerprint)
);

-- Configurações da seção Criptografia, em JSON, por conta.
CREATE TABLE account_crypto (
    account_id  TEXT PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    settings    TEXT NOT NULL
);

-- Tecnologia de uma mensagem criptografada ou assinada (pelo Content-Type do cabeçalho), para a lista
-- mostrar o cadeado sem abrir a mensagem.
ALTER TABLE message ADD COLUMN crypto TEXT CHECK (crypto IS NULL OR crypto IN ('pgp', 'smime'));

-- US3: esquema inicial do banco local (SQLCipher). Cada conta é dona de todas as linhas
-- abaixo dela; ON DELETE CASCADE garante que nada fica órfão se uma conta for apagada.

CREATE TABLE account (
    id            TEXT PRIMARY KEY,
    provider      TEXT NOT NULL,
    email         TEXT NOT NULL CHECK (email = lower(email)),
    display_name  TEXT NOT NULL CHECK (length(display_name) BETWEEN 1 AND 32),
    color         TEXT NOT NULL CHECK (color IN ('blue', 'teal', 'green', 'purple', 'magenta', 'orange')),
    position      INTEGER NOT NULL,
    status        TEXT NOT NULL CHECK (status IN ('active', 'needs_reauth', 'signed_out')),
    created_at    INTEGER NOT NULL,
    last_sync_at  INTEGER,
    UNIQUE (provider, email)
);

CREATE TABLE provider_config (
    account_id     TEXT PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    auth_kind      TEXT NOT NULL CHECK (auth_kind IN ('oauth2', 'password')),
    imap_host      TEXT NOT NULL,
    imap_port      INTEGER NOT NULL,
    imap_security  TEXT NOT NULL CHECK (imap_security IN ('tls', 'starttls')),
    smtp_host      TEXT NOT NULL,
    smtp_port      INTEGER NOT NULL,
    smtp_security  TEXT NOT NULL CHECK (smtp_security IN ('tls', 'starttls'))
);

CREATE TABLE folder (
    id             INTEGER PRIMARY KEY,
    account_id     TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    imap_name      TEXT NOT NULL,
    role           TEXT NOT NULL,
    display_name   TEXT NOT NULL,
    position       INTEGER NOT NULL,
    uidvalidity    INTEGER,
    uidnext        INTEGER,
    highestmodseq  INTEGER,
    -- Menor UID já trazido para o banco; NULL enquanto a pasta nunca foi sincronizada.
    oldest_uid     INTEGER,
    -- 1 quando o banco já tem a pasta inteira, até a mensagem mais antiga.
    complete       INTEGER NOT NULL DEFAULT 0,
    total          INTEGER,
    unread         INTEGER,
    last_sync_at   INTEGER,
    UNIQUE (account_id, imap_name)
);

CREATE TABLE message (
    id               INTEGER PRIMARY KEY,
    account_id       TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    -- Identifica a mesma mensagem em várias pastas (marcadores do Gmail): Message-ID + tamanho.
    dedupe_key       TEXT NOT NULL,
    subject          TEXT NOT NULL,
    from_name        TEXT NOT NULL,
    from_address     TEXT NOT NULL,
    to_json          TEXT NOT NULL,
    cc_json          TEXT NOT NULL,
    date             TEXT NOT NULL,
    size             INTEGER NOT NULL,
    seen             INTEGER NOT NULL,
    flagged          INTEGER NOT NULL,
    has_attachments  INTEGER NOT NULL,
    body_state       TEXT NOT NULL DEFAULT 'none' CHECK (body_state IN ('none', 'cached', 'too_large')),
    UNIQUE (account_id, dedupe_key)
);
CREATE INDEX ix_message_account_date ON message (account_id, date DESC);

CREATE TABLE message_folder (
    message_id  INTEGER NOT NULL REFERENCES message (id) ON DELETE CASCADE,
    folder_id   INTEGER NOT NULL REFERENCES folder (id) ON DELETE CASCADE,
    uid         INTEGER NOT NULL,
    PRIMARY KEY (folder_id, uid)
);
CREATE INDEX ix_message_folder_message ON message_folder (message_id);

-- Corpo RFC 822 completo. O HTML é sanitizado a cada leitura, nunca guardado já limpo.
CREATE TABLE message_body (
    message_id  INTEGER PRIMARY KEY REFERENCES message (id) ON DELETE CASCADE,
    raw         BLOB NOT NULL,
    fetched_at  INTEGER NOT NULL
);

CREATE TABLE attachment (
    id          INTEGER PRIMARY KEY,
    message_id  INTEGER NOT NULL REFERENCES message (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    mime_type   TEXT NOT NULL,
    size        INTEGER NOT NULL
);
CREATE INDEX ix_attachment_message ON attachment (message_id);

-- Marcadores/tags de provedores que os separam das pastas (categorias do Outlook, keywords IMAP).
-- No Gmail, cada marcador já é uma pasta em `folder`.
CREATE TABLE label (
    id          INTEGER PRIMARY KEY,
    account_id  TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    folder_id   INTEGER REFERENCES folder (id) ON DELETE SET NULL,
    color       TEXT,
    UNIQUE (account_id, name)
);

CREATE TABLE message_label (
    message_id  INTEGER NOT NULL REFERENCES message (id) ON DELETE CASCADE,
    label_id    INTEGER NOT NULL REFERENCES label (id) ON DELETE CASCADE,
    PRIMARY KEY (message_id, label_id)
);

CREATE TABLE account_setting (
    account_id  TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,
    PRIMARY KEY (account_id, key)
);

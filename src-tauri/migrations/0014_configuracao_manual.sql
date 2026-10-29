-- US22: configuração de entrada e saída por conta, com protocolo, método de
-- autenticação e usuário próprios. Contas existentes continuam com os mesmos
-- servidores; o Outlook passa a `graph` e as demais, a `imap`.

CREATE TABLE provider_config_new (
    account_id            TEXT PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    incoming_protocol     TEXT NOT NULL CHECK (incoming_protocol IN ('imap', 'pop3', 'graph')),
    incoming_host         TEXT NOT NULL,
    incoming_port         INTEGER NOT NULL CHECK (incoming_port BETWEEN 1 AND 65535),
    incoming_security     TEXT NOT NULL CHECK (incoming_security IN ('tls', 'starttls')),
    incoming_auth         TEXT NOT NULL CHECK (incoming_auth IN ('password', 'encrypted', 'gssapi', 'ntlm', 'oauth2')),
    incoming_username     TEXT NOT NULL,
    outgoing_host         TEXT NOT NULL,
    outgoing_port         INTEGER NOT NULL CHECK (outgoing_port BETWEEN 1 AND 65535),
    outgoing_security     TEXT NOT NULL CHECK (outgoing_security IN ('tls', 'starttls')),
    outgoing_auth         TEXT NOT NULL CHECK (outgoing_auth IN ('password', 'encrypted', 'gssapi', 'ntlm', 'oauth2')),
    outgoing_username     TEXT NOT NULL,
    outgoing_credentials  TEXT NOT NULL CHECK (outgoing_credentials IN ('same', 'separate'))
);

INSERT INTO provider_config_new (
    account_id, incoming_protocol, incoming_host, incoming_port, incoming_security,
    incoming_auth, incoming_username, outgoing_host, outgoing_port, outgoing_security,
    outgoing_auth, outgoing_username, outgoing_credentials
)
SELECT
    pc.account_id,
    CASE WHEN a.provider = 'outlook' THEN 'graph' ELSE 'imap' END,
    pc.imap_host,
    pc.imap_port,
    pc.imap_security,
    pc.auth_kind,
    a.email,
    pc.smtp_host,
    pc.smtp_port,
    pc.smtp_security,
    pc.auth_kind,
    a.email,
    'same'
FROM provider_config pc
JOIN account a ON a.id = pc.account_id;

DROP TABLE provider_config;
ALTER TABLE provider_config_new RENAME TO provider_config;

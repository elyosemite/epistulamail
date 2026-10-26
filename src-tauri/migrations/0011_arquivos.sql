-- US15: arquivos de e-mail importados (backups de outros programas), sem conta e sem servidor.
-- O arquivo local é uma linha de `account` com provider 'archive' e status 'signed_out' (o
-- processo Rust o trata como `Local`): nada de rede, sincronização, notificação ou regra o toca.

CREATE TABLE archive (
    account_id   TEXT PRIMARY KEY REFERENCES account (id) ON DELETE CASCADE,
    -- Formato reconhecido: takeout, mbox, thunderbird, apple_mail, maildir, eml, emlx, pst, msg.
    format       TEXT NOT NULL,
    -- Nome do arquivo ou da pasta de origem, para a tela. Nunca vai para o log.
    source_name  TEXT NOT NULL,
    -- Caminho de origem, para continuar uma importação interrompida (D5). Fica no banco cifrado.
    source_path  TEXT NOT NULL,
    -- Pastas escolhidas no passo 2 (JSON com as chaves de `mail_import::SourceFolder`).
    selected     TEXT NOT NULL,
    state        TEXT NOT NULL CHECK (state IN ('importing', 'interrupted', 'done')),
    imported     INTEGER NOT NULL DEFAULT 0,
    duplicates   INTEGER NOT NULL DEFAULT 0,
    -- Falhas por motivo (JSON: {"too_large": 12, "unreadable": 3}); nunca o conteúdo.
    failures     TEXT NOT NULL DEFAULT '{}',
    estimate     INTEGER NOT NULL DEFAULT 0,
    created_at   INTEGER NOT NULL,
    finished_at  INTEGER
);

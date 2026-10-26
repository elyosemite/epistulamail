-- US12: regras que organizam os e-mails ao chegar.

-- Lista de e-mails (List-Id; "-" quando só há List-Unsubscribe): condição "Lista de e-mails" e o
-- modelo Newsletters. Mensagens antigas ficam sem até serem baixadas de novo.
ALTER TABLE message ADD COLUMN list_id TEXT;

-- Uma regra por linha, na ordem em que roda. `spec` é o JSON com versão das pastas de origem,
-- condições, exceções, ações e "parar". `problem` é o motivo de estar desligada por erro.
CREATE TABLE rule (
    id          INTEGER PRIMARY KEY,
    account_id  TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    spec        TEXT NOT NULL,
    problem     TEXT,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE INDEX ix_rule_account ON rule (account_id, position);

-- Histórico (CA6): um lote por regra e por execução, com o nome guardado para sobreviver à regra.
CREATE TABLE rule_run (
    id          INTEGER PRIMARY KEY,
    account_id  TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    rule_id     INTEGER REFERENCES rule (id) ON DELETE SET NULL,
    rule_name   TEXT NOT NULL,
    at          INTEGER NOT NULL,
    origin      TEXT NOT NULL CHECK (origin IN ('sync', 'manual', 'cleanup')),
    summary     TEXT NOT NULL,
    count       INTEGER NOT NULL,
    undone      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX ix_rule_run_account ON rule_run (account_id, at DESC);

-- O que cada lote fez em cada mensagem, para desfazer: de onde saiu, para onde foi e as flags de antes.
CREATE TABLE rule_hit (
    run_id          INTEGER NOT NULL REFERENCES rule_run (id) ON DELETE CASCADE,
    message_id      INTEGER NOT NULL REFERENCES message (id) ON DELETE CASCADE,
    from_folder_id  INTEGER NOT NULL,
    to_folder_id    INTEGER,
    seen_before     INTEGER NOT NULL,
    flagged_before  INTEGER NOT NULL,
    PRIMARY KEY (run_id, message_id)
);

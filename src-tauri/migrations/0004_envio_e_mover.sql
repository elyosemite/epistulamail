-- US7: mover pendente (o "Desfazer" fica no backend), prévia da lista e nome de quem envia.

-- Mensagem a caminho de outra pasta. Some da origem e aparece no destino na hora; o MOVE vai ao
-- servidor quando o prazo do "Desfazer" acaba (state: waiting -> running -> moved).
CREATE TABLE pending_move (
    id              INTEGER PRIMARY KEY,
    batch           TEXT NOT NULL,
    account_id      TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    message_id      INTEGER NOT NULL REFERENCES message (id) ON DELETE CASCADE,
    from_folder_id  INTEGER NOT NULL REFERENCES folder (id) ON DELETE CASCADE,
    from_uid        INTEGER NOT NULL,
    to_folder_id    INTEGER NOT NULL REFERENCES folder (id) ON DELETE CASCADE,
    state           TEXT NOT NULL CHECK (state IN ('waiting', 'running', 'moved')),
    due_at          INTEGER NOT NULL,
    moved_at        INTEGER,
    UNIQUE (from_folder_id, from_uid)
);
CREATE INDEX ix_pending_move_to ON pending_move (to_folder_id);
CREATE INDEX ix_pending_move_batch ON pending_move (batch);

-- Prévia da densidade Confortável: NULL enquanto a estrutura da mensagem não foi lida.
ALTER TABLE message ADD COLUMN snippet TEXT;
ALTER TABLE message ADD COLUMN has_files INTEGER NOT NULL DEFAULT 0;
ALTER TABLE message ADD COLUMN has_images INTEGER NOT NULL DEFAULT 0;

-- Nome do titular da conta (claim `name`), usado no cabeçalho From do que for enviado.
ALTER TABLE account ADD COLUMN sender_name TEXT;

-- US4: pastas recentes do "Mover para", sinalização com prazo, rascunhos locais e mensagens em
-- trânsito entre pastas.

-- Última vez que o usuário moveu algo para cada pasta (as 5 mais recentes aparecem no menu).
CREATE TABLE folder_usage (
    account_id  TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    folder_id   INTEGER NOT NULL REFERENCES folder (id) ON DELETE CASCADE,
    used_at     INTEGER NOT NULL,
    PRIMARY KEY (account_id, folder_id)
);

-- Prazo de acompanhamento. O Gmail só guarda a estrela; prazo e conclusão ficam aqui.
CREATE TABLE message_followup (
    message_id    INTEGER PRIMARY KEY REFERENCES message (id) ON DELETE CASCADE,
    due_date      TEXT CHECK (due_date IS NULL OR due_date GLOB '[0-9][0-9][0-9][0-9]-[0-1][0-9]-[0-3][0-9]'),
    completed_at  INTEGER
);

-- Rascunhos locais de resposta, encaminhamento e e-mail novo (o envio chega numa story futura).
CREATE TABLE draft (
    id           INTEGER PRIMARY KEY,
    account_id   TEXT NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN ('new', 'reply', 'reply_all', 'forward')),
    in_reply_to  INTEGER REFERENCES message (id) ON DELETE CASCADE,
    to_json      TEXT NOT NULL,
    cc_json      TEXT NOT NULL,
    bcc_json     TEXT NOT NULL,
    subject      TEXT NOT NULL,
    body         TEXT NOT NULL,
    updated_at   INTEGER NOT NULL,
    CHECK ((kind = 'new') = (in_reply_to IS NULL))
);
CREATE UNIQUE INDEX ux_draft_reply ON draft (account_id, kind, in_reply_to) WHERE in_reply_to IS NOT NULL;
CREATE UNIQUE INDEX ux_draft_new ON draft (account_id) WHERE kind = 'new';

-- Mensagem movida que ainda não apareceu na pasta de destino: não é apagada como órfã por 10 min,
-- para manter corpo salvo, prazo e rascunho quando a sincronização do destino a encontrar.
ALTER TABLE message ADD COLUMN in_transit_since INTEGER;

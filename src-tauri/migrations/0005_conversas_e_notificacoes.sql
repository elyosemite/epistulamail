-- US8: conversas (cabeçalhos de resposta) e preferências de notificação.

-- Message-ID, In-Reply-To + References (ids sem <>, em minúsculas, separados por espaço) e a raiz da
-- conversa. thread_id NULL: cabeçalhos ainda não lidos; '': a mensagem não tem nenhum id (fica sozinha).
ALTER TABLE message ADD COLUMN msgid TEXT;
ALTER TABLE message ADD COLUMN refs TEXT;
ALTER TABLE message ADD COLUMN thread_id TEXT;
CREATE INDEX ix_message_thread ON message (account_id, thread_id);
CREATE INDEX ix_message_msgid ON message (account_id, msgid);

-- Histórico do sino (CA7, D12): as notificações dos últimos 7 dias, com o texto já respeitando a
-- privacidade escolhida. kind: mail, reply, summary, due, failure, reauth, offline. target: JSON
-- com a conta e a mensagem do clique.
CREATE TABLE notice (
    id          INTEGER PRIMARY KEY,
    account_id  TEXT REFERENCES account (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    title       TEXT NOT NULL,
    line1       TEXT NOT NULL,
    line2       TEXT NOT NULL,
    sender      TEXT,
    target      TEXT,
    created_at  INTEGER NOT NULL,
    seen        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX ix_notice_created ON notice (created_at);

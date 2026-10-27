-- US16: buscas salvas (aparecem na coluna de pastas). account_id NULL: busca em todas as contas.
CREATE TABLE saved_search (
    id          INTEGER PRIMARY KEY,
    account_id  TEXT REFERENCES account (id) ON DELETE CASCADE,
    name        TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 60),
    -- A consulta da busca (JSON de `search::SearchQuery`), com os chips como foram montados.
    query       TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);
CREATE INDEX ix_saved_search_account ON saved_search (account_id);

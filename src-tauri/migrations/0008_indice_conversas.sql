-- Desempenho: as consultas de uma conversa ordenam por data. Com o índice só por (conta, conversa),
-- o SQLite preferia o índice por (conta, data) e percorria todas as mensagens da conta para cada
-- conversa da lista (mais de 1 s numa página de 50). Este índice atende o filtro e a ordem.
CREATE INDEX ix_message_thread_date ON message (account_id, thread_id, date);
DROP INDEX ix_message_thread;

-- US20: Outlook pessoal via Microsoft Graph. O id remoto da mensagem fica ao lado do UID sintético
-- local, e a pasta guarda o link da próxima página de mensagens mais antigas (e o delta, quando
-- existir). O Gmail continua preenchendo só o UID.
ALTER TABLE message_folder ADD COLUMN remote_id TEXT;
CREATE UNIQUE INDEX ux_message_folder_remote ON message_folder (folder_id, remote_id) WHERE remote_id IS NOT NULL;
ALTER TABLE folder ADD COLUMN graph_next_link TEXT;
ALTER TABLE folder ADD COLUMN graph_delta_link TEXT;

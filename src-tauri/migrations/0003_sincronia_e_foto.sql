-- US5: intervalo de sincronização (padrão do app e por conta) e foto da conta.

-- Ajustes do app que não pertencem a uma conta (ex.: sync_default_minutes).
CREATE TABLE app_setting (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);

-- Foto do perfil do provedor, baixada pelo processo Rust (até 256 KB).
ALTER TABLE account ADD COLUMN photo BLOB;
ALTER TABLE account ADD COLUMN photo_type TEXT CHECK (photo_type IS NULL OR photo_type IN ('image/jpeg', 'image/png', 'image/webp'));
ALTER TABLE account ADD COLUMN photo_checked_at INTEGER;

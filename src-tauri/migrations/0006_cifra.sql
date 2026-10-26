-- US10: versão do esquema de cifra do banco (CA4). 1 = SQLCipher 4 com chave crua de 256 bits.
INSERT INTO app_setting (key, value) VALUES ('cipher_scheme', '1') ON CONFLICT (key) DO NOTHING;

# Epistula: arquitetura e modelo de ameaças

Situação em 23 out 2026, após a US4. Produto: cliente desktop que conecta várias contas de e-mail (Gmail por OAuth nesta versão) e lê as caixas por IMAP, com um banco local cifrado (decisão D1, opção A, provisória). Não há servidor do Epistula.

## Componentes e limites de confiança

```
┌──────────────────────────── Computador do usuário ────────────────────────────┐
│                                                                               │
│  WebView (React)  ──IPC (comandos e eventos Tauri)──▶  Processo Rust          │
│   · só exibe dados                                     · OAuth (PKCE, loopback)│
│   · nunca vê tokens nem a chave do banco               · IMAP XOAUTH2 + TLS    │
│   · HTML de e-mail num iframe                          · parse MIME + limpeza  │
│     sandbox sem scripts                                · até 2 sessões IMAP    │
│                                                          por conta, em memória │
│                                                        · sincronização         │
│                                        │                       │               │
│                 Keyring do SO ◀────────┘                       ▼               │
│                 · refresh token por conta          epistula.db (SQLCipher,     │
│                 · chave do banco (db-key)          AES-256) em app_data_dir    │
│                 · índice de contas                                            │
└────────────────────────────────────────────────────────┬──────────────────────┘
                                                         │ TLS
                       ┌─────────────────────────────────┴───────────────┐
                       │ Google: accounts.google.com, oauth2.googleapis.com│
                       │ (token e revogação), imap.gmail.com:993          │
                       └──────────────────────────────────────────────────┘
```

Limites de confiança, do mais confiável ao menos:

1. **Processo Rust.** Guarda credenciais e a chave do banco, decide o que sai para a WebView e o que vai para o log.
2. **Banco local.** Escrito só pelo processo Rust, cifrado em disco. O HTML é guardado cru e sanitizado a cada leitura, para que uma correção do sanitizador valha para mensagens antigas.
3. **WebView do app.** Código do Epistula. Recebe só dados de exibição e ids opacos; a CSP proíbe scripts externos e `eval`.
4. **Conteúdo de e-mail de terceiros.** Não confiável. Cabeçalhos são texto; HTML passa pelo `ammonia` no Rust e é exibido num `<iframe sandbox>` sem `allow-scripts`, com CSP própria.
5. **Google.** Provedor externo. Tem acesso a todo o conteúdo da conta, por definição.

## Matriz de dados

| Dado | Onde fica | Por quanto tempo | Quem acessa |
|---|---|---|---|
| Refresh token OAuth (por conta) | Keyring do SO (`epistula/account-<id>` + `secret-<id>-N`) | Até sair da conta, `invalid_grant` ou 7 dias (app em modo Testing) | Processo Rust; usuário do SO |
| Chave do banco (32 bytes do gerador do SO) | Keyring do SO (`epistula/db-key`) | Permanente | Processo Rust; usuário do SO |
| Índice de contas (id, provedor, e-mail) | Keyring do SO (`epistula/accounts-index`) | Permanente | Processo Rust; usuário do SO |
| Access token OAuth | Memória do processo Rust | ~1 h ou até fechar o app | Processo Rust |
| Sessões IMAP autenticadas | Memória do processo Rust (até 2 por conta) | Até erro, saída da conta ou fechar o app | Processo Rust |
| Contas (e-mail, nome dado, cor, ordem, status), pastas, marcadores, ajustes | `epistula.db` (SQLCipher) | Permanente, inclusive depois de sair da conta (D4) | Processo Rust; WebView (exibição) |
| Cabeçalhos (assunto, remetente, destinatários, data, flags) | `epistula.db` | As 2.000 mais recentes de cada pasta, mais as paginadas ou abertas (D5); saem quando somem do servidor | Processo Rust; WebView |
| Corpo RFC 822 das mensagens abertas e metadados de anexos | `epistula.db` | Até a mensagem sumir do servidor | Processo Rust; WebView (HTML sanitizado) |
| Rascunhos de resposta, encaminhamento e e-mail novo (destinatários, assunto, texto) | `epistula.db` (`draft`) | Até o usuário descartar ou a mensagem respondida sair do banco | Processo Rust; WebView |
| Prazos de sinalização e pastas usadas recentemente | `epistula.db` (`message_followup`, `folder_usage`) | Até limpar a sinalização ou a pasta sumir | Processo Rust; WebView |
| Largura da lista e última pasta, por conta | `epistula.db` (`account_setting`) | Permanente | Processo Rust; WebView |
| `client_id` e `client_secret` do Google | `.env` local (dev) ou binário (build) | Permanente | Qualquer pessoa com o binário (o Google não trata o secret de app desktop como confidencial) |
| Logs | stdout do processo | Sessão do terminal | Usuário do SO |

Os logs contêm só: comando, papel da pasta, contagens, tamanhos, duração, códigos de erro e status de conta. Termos de busca, nomes de pasta criados e rascunhos nunca vão para o log. A conta nunca aparece por e-mail ou nome; erros do SQLite vão só com o código. O teste `message::tests::logs_never_contain_message_content` captura os logs da suíte inteira, passa por contas, pastas, mensagens, corpo e saída de conta com marcadores únicos, e falha se algum aparecer. O teste `store::tests::database_file_has_no_plaintext` confere que o arquivo do banco (e o WAL) não contém nenhum desses marcadores.

## Esquema do banco

```
account ──┬── provider_config (1:1)
          ├── account_setting (última pasta aberta)
          ├── folder ──────────────┐
          ├── label ── message_label
          └── message ─── message_folder ──┘   (mensagem ↔ pastas, com o UID em cada uma)
                 ├── message_body (RFC 822 cru)
                 └── attachment (só metadados)
```

- Toda tabela pende de `account` com `ON DELETE CASCADE`.
- `UNIQUE(provider, email)` em `account`.
- `UNIQUE(account_id, dedupe_key)` em `message`: no Gmail, a mesma mensagem em vários marcadores é guardada uma vez (Message-ID + tamanho; sem Message-ID, fica restrita à pasta e ao UID).
- Migrações em `src-tauri/migrations/`, aplicadas em transação e controladas por `PRAGMA user_version`. `foreign_keys`, `secure_delete`, WAL e `temp_store = MEMORY` ligados ao abrir.

## Ameaças

| # | Ameaça | Situação | Controles e riscos residuais |
|---|---|---|---|
| 1 | Atacante externo sem autenticação | Mitigada | Não há porta aberta além do loopback efêmero do OAuth: 127.0.0.1, aceita uma requisição, valida `state` de uso único e expira em 5 min. |
| 2 | Usuário autenticado acessando dados de outra conta | Mitigada no app | Várias contas por instalação, todas do mesmo usuário do SO. Ids opacos carregam a conta, e o backend recusa (`InvalidId`) um id de pasta ou mensagem que não pertença à conta do pedido. O isolamento no provedor é do Google. |
| 3 | Roubo de banco ou backup | Mitigada | `epistula.db` é cifrado por inteiro com SQLCipher (AES-256, HMAC por página), inclusive índices e metadados. A chave fica só no keyring. Uma cópia do arquivo, sozinha, não revela nada. Backups do perfil do SO que incluam o keyring levam a chave junto, cifrada pelo SO. |
| 4 | Conta administrativa comprometida | Fora do escopo | Não há administração do Epistula. Um admin de Workspace pode revogar ou auditar o acesso pelo Google. |
| 5 | CI/CD ou cadeia de dependências | Parcialmente mitigada | `Cargo.lock` e `pnpm-lock.yaml` versionados; `cargo deny` (licenças, fontes, advisories) e `cargo audit`. O OpenSSL do SQLCipher é compilado do fonte (`openssl-src`); o Strawberry Perl usado no build é baixado por `scripts/setup-build-tools.ps1` com SHA-256 fixo. Ainda não há CI nem auditoria das dependências npm, e os releases não são assinados. |
| 6 | Operador curioso ou malicioso | Fora do escopo | Não há operador do Epistula com acesso a dados. O Google, como provedor, tem acesso total (ver 8). |
| 7 | Dispositivo do usuário comprometido | Não mitigada | Malware com a sessão do usuário lê o keyring (refresh tokens e chave do banco), o banco aberto, a memória e a tela. Nenhum cliente de e-mail resolve isso. |
| 8 | Provedor externo sem E2EE | Aceita | **O Google lê todos os e-mails da conta.** O Epistula não muda isso; TLS protege só o trânsito. E2EE depende da decisão D3 (OpenPGP no cliente). |
| 9 | Abuso: phishing, rastreamento, conteúdo malicioso | Parcialmente mitigada | HTML limpo por allowlist no Rust a cada leitura (corpus de ataques + proptest); iframe sem scripts; imagens remotas bloqueadas até o usuário liberar por mensagem; links só `http`/`https`/`mailto`, abertos no navegador do sistema. Não há detecção de phishing nem verificação de SPF/DKIM na UI. |
| 10 | Perda, roubo ou revogação de chaves | Mitigada | Sair da conta revoga o refresh token no Google e apaga o segredo do keyring. Revogar em `myaccount.google.com/permissions` faz a conta virar "precisa reconectar" sem apagar os dados. Sem a chave do banco, o arquivo é posto de lado (`epistula.db.unreadable-*`) e um banco novo é criado; as contas voltam pelo índice do keyring e o conteúdo é baixado de novo do servidor. Não há chaves E2EE ainda. |
| 11 | Logs, traces, dumps de memória, temporários | Parcialmente mitigada | Logs sem conteúdo nem identificação da conta, com teste. WAL cifrado como o banco; `temp_store = MEMORY` evita temporários em disco; `secure_delete` sobrescreve páginas apagadas. Dumps de memória do processo conteriam tokens, a chave do banco e mensagens abertas; os segredos não são zerados ao sair da memória. |
| 12 | Infraestrutura ou ordem legal | Fora do escopo do Epistula | Não há infraestrutura do Epistula. Pedidos legais alcançam os dados no Google. |

**Risco aceito pela D4.** Depois de sair de uma conta, as mensagens salvas dela continuam no disco (cifradas). Qualquer pessoa com acesso à sessão do usuário no Windows consegue lê-las pelo Epistula. Apagar os dados de uma conta do computador fica para outra story.

## Premissas

- O keyring do SO protege os itens da sessão do usuário contra outros usuários locais.
- A WebView do sistema (WebView2 no Windows) aplica corretamente `sandbox`, `srcdoc` e CSP.
- O `ammonia` (allowlist sobre o parser html5ever) é a barreira principal contra HTML ativo; o iframe sem scripts é a segunda.
- O SQLCipher 4 com as configurações padrão (AES-256-CBC, HMAC-SHA512, chave crua sem KDF, porque já vem do gerador do SO) é adequado para cifra em repouso.
- Testes contra o Gmail real são manuais: não há servidor IMAP com XOAUTH2 em CI. O banco tem testes de integração reais (SQLCipher em diretório temporário).

## Riscos residuais priorizados

1. Releases sem assinatura nem atualização verificada (ameaça 5): antes de distribuir.
2. Verificação do app OAuth pelo Google, com avaliação de segurança, para sair do modo Testing.
3. CI com `cargo clippy`, testes, `cargo deny` e `cargo audit`, mais auditoria das dependências npm.
4. Dados de contas de que o usuário saiu continuam no disco (D4): oferecer "apagar dados desta conta".
5. Indicadores de autenticidade do remetente (SPF/DKIM/DMARC dos cabeçalhos `Authentication-Results`).

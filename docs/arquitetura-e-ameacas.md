# Epistula: arquitetura e modelo de ameaças

Situação em 25 out 2026, após a US7. Produto: cliente desktop que conecta várias contas de e-mail (Gmail por OAuth nesta versão), lê as caixas por IMAP e envia por SMTP, com um banco local cifrado (decisão D1, opção A, provisória). Não há servidor do Epistula.

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
│                                                        · sincronização por     │
│                                                          conta (intervalo)     │
│                                                        · foto do perfil        │
│                                                        · SMTP XOAUTH2 + TLS    │
│                                                          (envio, US7)          │
│                                                        · mover com "Desfazer"  │
│                                                          executado no backend  │
│                                        │                       │               │
│                 Keyring do SO ◀────────┘                       ▼               │
│                 · refresh token por conta          epistula.db (SQLCipher,     │
│                 · chave do banco (db-key)          AES-256) em app_data_dir    │
│                 · índice de contas                                            │
└────────────────────────────────────────────────────────┬──────────────────────┘
                                                         │ TLS
                       ┌─────────────────────────────────┴───────────────┐
                       │ Google: accounts.google.com, oauth2.googleapis.com│
                       │ (token e revogação), imap.gmail.com:993,         │
                       │ smtp.gmail.com:465 (envio, TLS implícito),       │
                       │ *.googleusercontent.com (foto do perfil)         │
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
| Refresh token OAuth (por conta) | Keyring do SO (`epistula/account-<id>` + `secret-<id>-N`); com a senha do Epistula ligada, cifrado por ela (AES-256-GCM, prefixo `sealed1:`) | Até sair da conta, `invalid_grant` ou 7 dias (app em modo Testing) | Processo Rust; usuário do SO |
| Chave do banco (32 bytes do gerador do SO) | Keyring do SO (`epistula/db-key`, em hexadecimal: formato v1; o v2, envolvido pela senha do Epistula, vem com a US10) | Até a próxima troca de chave | Processo Rust; usuário do SO |
| Chave derivada da senha do Epistula (Argon2id) | Memória do processo Rust, só com o app desbloqueado | Até bloquear ou fechar o app | Processo Rust |
| Chave nova durante uma troca | Keyring do SO (`epistula/db-key-next`) | Do início da troca até o banco abrir com ela (segundos); uma troca interrompida é concluída ou desfeita na próxima abertura | Processo Rust; usuário do SO |
| Índice de contas (id, provedor, e-mail) | Keyring do SO (`epistula/accounts-index`) | Permanente | Processo Rust; usuário do SO |
| Access token OAuth | Memória do processo Rust | ~1 h ou até fechar o app | Processo Rust |
| Sessões IMAP autenticadas | Memória do processo Rust (até 2 por conta) | Até erro, saída da conta ou fechar o app | Processo Rust |
| Contas (e-mail, nome dado, cor, ordem, status), pastas, marcadores, ajustes | `epistula.db` (SQLCipher) | Permanente, inclusive depois de sair da conta (D4) | Processo Rust; WebView (exibição) |
| Cabeçalhos (assunto, remetente, destinatários, data, flags) | `epistula.db` | As 2.000 mais recentes de cada pasta, mais as paginadas ou abertas (D5); saem quando somem do servidor | Processo Rust; WebView |
| Corpo RFC 822 das mensagens abertas e metadados de anexos | `epistula.db` | Até a mensagem sumir do servidor | Processo Rust; WebView (HTML sanitizado) |
| Rascunhos de resposta, encaminhamento e e-mail novo (destinatários, assunto, texto) | `epistula.db` (`draft`) | Até o usuário descartar, a mensagem ser enviada ou a mensagem respondida sair do banco | Processo Rust; WebView |
| Mensagem a enviar no prazo do "Desfazer envio" (destinatários, assunto, texto, anexos encaminhados) | Memória do processo Rust | 6 s, até sair para o servidor ou ser desfeita; fechar o app nesse prazo não envia (o rascunho continua) | Processo Rust |
| Trecho do início do texto (até 200 caracteres) e se há arquivos ou imagens anexados | `epistula.db` (`message.snippet`, `has_files`, `has_images`) | Como os cabeçalhos | Processo Rust; WebView |
| Mover pendente (mensagem, pasta de origem e de destino, prazo) | `epistula.db` (`pending_move`) | Do mover até a pasta de destino ser sincronizada (no máximo 10 min depois do MOVE) | Processo Rust |
| Nome do titular da conta (claim `name`), para o `From` do envio | `epistula.db` (`account.sender_name`) | Atualizado a cada renovação do token | Processo Rust |
| Prazos de sinalização e pastas usadas recentemente | `epistula.db` (`message_followup`, `folder_usage`) | Até limpar a sinalização ou a pasta sumir | Processo Rust; WebView |
| Largura da lista, última pasta e intervalo de sincronização, por conta | `epistula.db` (`account_setting`) | Permanente | Processo Rust; WebView |
| Intervalo padrão de sincronização, tema, cor do fundo, densidade da lista e colunas visíveis | `epistula.db` (`app_setting`) | Permanente | Processo Rust; WebView |
| Cópia da aparência e das preferências de lista (só esses valores) | Armazenamento local da WebView | Até a próxima mudança | WebView |
| Foto do perfil Google (até 256 KB, JPEG/PNG/WebP) | `epistula.db` (`account.photo`) | Até a conta ser removida; trocada a cada conexão e no máximo uma vez por dia | Processo Rust; WebView (como `data:` URL) |
| `client_id` e `client_secret` do Google | `.env` local (dev) ou binário (build) | Permanente | Qualquer pessoa com o binário (o Google não trata o secret de app desktop como confidencial) |
| Logs | stdout do processo | Sessão do terminal | Usuário do SO |

Os logs contêm só: comando, papel da pasta, contagens, tamanhos, duração, códigos de erro e status de conta. Termos de busca, nomes de pasta criados, rascunhos, trechos da lista, a URL e os bytes da foto nunca vão para o log. No envio, só a quantidade de destinatários e o resultado (`mensagem enviada`, `envio desfeito`, ou o tipo de erro e o código SMTP): nunca destinatários, assunto, corpo ou nomes de anexos. A conta nunca aparece por e-mail ou nome; erros do SQLite vão só com o código. O teste `message::tests::logs_never_contain_message_content` captura os logs da suíte inteira, passa por contas, pastas, mensagens, corpo e saída de conta com marcadores únicos, e falha se algum aparecer. O teste `store::tests::database_file_has_no_plaintext` confere que o arquivo do banco (e o WAL) não contém nenhum desses marcadores.

## Esquema do banco

```
app_setting (intervalo padrão de sincronização, aparência, densidade, colunas visíveis)

account ──┬── provider_config (1:1)       (account.photo: foto do perfil)
          ├── account_setting (última pasta, largura da lista, intervalo de sincronização)
          ├── folder ──────────────┐
          ├── label ── message_label
          ├── message ─── message_folder ──┘   (mensagem ↔ pastas, com o UID em cada uma)
          │      ├── message_body (RFC 822 cru)
          │      └── attachment (só metadados)
          └── pending_move (mensagem a caminho de outra pasta: origem, destino, estado, prazo)
```

- Toda tabela pende de `account` com `ON DELETE CASCADE`.
- `UNIQUE(provider, email)` em `account`.
- `UNIQUE(account_id, dedupe_key)` em `message`: no Gmail, a mesma mensagem em vários marcadores é guardada uma vez (Message-ID + tamanho; sem Message-ID, fica restrita à pasta e ao UID).
- Migrações em `src-tauri/migrations/`, aplicadas em transação e controladas por `PRAGMA user_version`. `foreign_keys`, `secure_delete`, WAL e `temp_store = MEMORY` ligados ao abrir.
- As configurações de cifra são fixas no código (`CIPHER_PRAGMAS` em `db.rs`), iguais ao padrão do SQLCipher 4, e um teste falha se mudarem. `app_setting.cipher_scheme` guarda a versão do esquema de cifra (1); um banco com versão maior que a conhecida não abre.

## Criptografia

Inventário de cada uso de criptografia (US10, CA1). "Quântico" diz se o uso resiste a um computador quântico capaz de quebrar RSA e curvas elípticas.

| Onde | Para quê | Algoritmo e biblioteca | Tipo | Quântico | O que fazer |
|---|---|---|---|---|---|
| `epistula.db` e WAL | Cifra em repouso de tudo que o app guarda | SQLCipher 4: AES-256-CBC por página, HMAC-SHA512, chave crua de 256 bits (OpenSSL embutido) | Simétrica | Resiste: o algoritmo de Grover deixa cerca de 128 bits | Nada. Não existe cifra simétrica "pós-quântica" melhor (D10 da US10) |
| Chave do banco e refresh tokens no cofre | Guardar os segredos fora do banco | DPAPI do Windows (`keyring`): chave derivada da senha do Windows | Simétrica | Resiste | Quem tem a sessão ou a senha do Windows abre o cofre: senha do Epistula (US10, CA2) |
| Backup do DPAPI em domínio | O administrador do domínio recupera o cofre de qualquer usuário | RSA no controlador de domínio | Chave pública | Não resiste | Senha do Epistula (US10, CA2): sem ela, a chave do cofre não abre o banco |
| Troca da chave do banco | Aposentar uma chave | `PRAGMA rekey` do SQLCipher, numa transação | Simétrica | Resiste | Feito (`keys.rs`) |
| TLS de IMAP, SMTP, login e foto | Sigilo e autenticidade no trânsito | `native-tls` (SChannel): troca de chaves ECDHE, certificados RSA ou ECDSA | Chave pública | A troca de chaves não resiste ("colher agora, decifrar depois"). O certificado só importa no momento da conexão | Troca híbrida X25519 + ML-KEM-768 (US11) |
| OAuth PKCE | Amarrar o código de autorização ao app | SHA-256 (`sha2`) | Hash | Resiste | Nada |
| `state` do OAuth, chave do banco, `code_verifier` | Valores imprevisíveis | Gerador do sistema (`OsRng`) | | Resiste | Nada |
| Senha do Epistula (US10, CA2) | Envolver a chave do banco e os refresh tokens | Argon2id (RFC 9106, 64 MiB, 3 passadas) + AES-256-GCM (`argon2`, `aes-gcm`) | Simétrica | Resiste | Feito (`lock.rs`) |

**Regra para recursos futuros (US10, CA10).** Todo recurso que precisar de chave pública (backup aberto em outro computador, sincronização entre dispositivos, e-mail cifrado entre pessoas) usa o modelo híbrido, X25519 + ML-KEM-768 via HPKE (RFC 9180) ou X-Wing quando estiver padronizado, com biblioteca auditada. Assinaturas, quando forem necessárias, consideram ML-DSA (FIPS 204) em modo híbrido. Nenhuma criptografia própria.

## Ameaças

| # | Ameaça | Situação | Controles e riscos residuais |
|---|---|---|---|
| 1 | Atacante externo sem autenticação | Mitigada | Não há porta aberta além do loopback efêmero do OAuth: 127.0.0.1, aceita uma requisição, valida `state` de uso único e expira em 5 min. |
| 2 | Usuário autenticado acessando dados de outra conta | Mitigada no app | Várias contas por instalação, todas do mesmo usuário do SO. Ids opacos carregam a conta, e o backend recusa (`InvalidId`) um id de pasta ou mensagem que não pertença à conta do pedido. O isolamento no provedor é do Google. |
| 3 | Roubo de banco ou backup | Mitigada | `epistula.db` é cifrado por inteiro com SQLCipher (AES-256, HMAC por página), inclusive índices e metadados. A chave fica só no keyring. Uma cópia do arquivo, sozinha, não revela nada. Backups do perfil do SO que incluam o keyring levam a chave junto, cifrada pelo SO (ver 15). |
| 4 | Conta administrativa comprometida | Fora do escopo | Não há administração do Epistula. Um admin de Workspace pode revogar ou auditar o acesso pelo Google. |
| 5 | CI/CD ou cadeia de dependências | Parcialmente mitigada | `Cargo.lock` e `pnpm-lock.yaml` versionados; `cargo deny` (licenças, fontes, advisories) e `cargo audit` locais. O OpenSSL do SQLCipher é compilado do fonte (`openssl-src`); o Strawberry Perl usado no build é baixado por `scripts/setup-build-tools.ps1` com SHA-256 fixo. O CI (GitHub Actions) roda build, clippy, testes e gitleaks, sem segredos. Ainda não há auditoria contínua das dependências, e os releases não são assinados. |
| 6 | Operador curioso ou malicioso | Fora do escopo | Não há operador do Epistula com acesso a dados. O Google, como provedor, tem acesso total (ver 8). |
| 7 | Dispositivo do usuário comprometido | Não mitigada | Malware com a sessão do usuário lê o keyring (refresh tokens e chave do banco), o banco aberto, a memória e a tela. Nenhum cliente de e-mail resolve isso. |
| 8 | Provedor externo sem E2EE | Aceita | **O Google lê todos os e-mails da conta.** O Epistula não muda isso; TLS protege só o trânsito. E2EE depende da decisão D3 (OpenPGP no cliente). |
| 9 | Abuso: phishing, rastreamento, conteúdo malicioso | Parcialmente mitigada | HTML limpo por allowlist no Rust a cada leitura (corpus de ataques + proptest); iframe sem scripts; imagens remotas bloqueadas até o usuário liberar por mensagem; links só `http`/`https`/`mailto`, abertos no navegador do sistema. A foto do perfil vem da claim `picture` do `id_token` e só é baixada de `https://*.googleusercontent.com` (também depois de redirecionamentos), com até 256 KB, timeout de 10 s e só JPEG, PNG ou WebP (SVG recusado); a WebView recebe uma `data:` URL e não faz requisições ao Google. Não há detecção de phishing nem verificação de SPF/DKIM na UI. A prévia da lista lê só a estrutura e os primeiros 2 KB da parte de texto (`BODY.PEEK`, sem marcar como lida); o HTML vira texto puro no Rust, e o charset que vai para o cabeçalho montado é limpo. |
| 13 | Envio: injeção de cabeçalho, vazamento do Cco e da identidade do computador | Mitigada | A mensagem é montada pela `lettre` (cabeçalhos codificados, endereços validados); controles no assunto viram espaço, com teste de injeção. O Cco vai só no envelope SMTP, nunca nos cabeçalhos (teste). O `Message-ID` usa o domínio de quem envia e o EHLO não leva o nome do computador (sem a feature `hostname`). TLS implícito na porta 465 com certificado validado; o token só segue depois do TLS. Até 100 destinatários, 2 MB de texto e 25 MB com anexos. Enviar exige a conta ativa. |
| 14 | Mover: perda de mensagem ou estado inconsistente | Mitigada | O mover fica registrado no banco antes de ir ao servidor; se o app fechar no prazo do "Desfazer" ou no meio do MOVE, ele é refeito na próxima abertura (refazer um MOVE já feito não faz nada no servidor). Se o servidor recusar, a mensagem volta para a origem e o usuário é avisado. Enquanto espera a sincronização do destino, a mensagem fica protegida da limpeza de órfãs por até 10 min. |
| 10 | Perda, roubo ou revogação de chaves | Mitigada | Sair da conta revoga o refresh token no Google e apaga o segredo do keyring. Revogar em `myaccount.google.com/permissions` faz a conta virar "precisa reconectar" sem apagar os dados. Sem a chave do banco, o arquivo é posto de lado (`epistula.db.unreadable-*`) e um banco novo é criado; as contas voltam pelo índice do keyring e o conteúdo é baixado de novo do servidor. Não há chaves E2EE ainda. |
| 11 | Logs, traces, dumps de memória, temporários | Parcialmente mitigada | Logs sem conteúdo nem identificação da conta, com teste. WAL cifrado como o banco; `temp_store = MEMORY` evita temporários em disco; `secure_delete` sobrescreve páginas apagadas. A chave do banco, os refresh tokens e os access tokens ficam em `Zeroizing` e são zerados ao sair de uso; a chave nunca aparece em `Debug` (teste). As cópias feitas dentro das bibliotecas (`keyring`, SChannel, `lettre`, o SQLCipher) não são zeradas pelo app; o `cipher_memory_security` do SQLCipher estoura a pilha no Windows e fica desligado. O processo Rust sai dos relatórios de erro do Windows (`WerAddExcludedApplication`), para uma falha não gravar um despejo de memória; os processos do WebView2, que mostram o e-mail aberto, ainda têm os próprios relatórios (US10, CA6). |
| 15 | Cópia do disco com a senha do Windows, ou backup do DPAPI em domínio | Mitigada com a senha do Epistula ligada (opcional, D1) | Sem senha, o DPAPI entrega a chave do banco e os refresh tokens a quem está na sessão do usuário, a quem copia o disco e tem a senha do Windows (que, sem BitLocker, pode ser adivinhada offline) e ao administrador do domínio, que tem a chave de backup do DPAPI (protegida por RSA). Com a senha, a chave do banco e os refresh tokens ficam no cofre cifrados por uma chave derivada dela com Argon2id; sem a senha, nem o banco abre nem o token dá acesso ao servidor. Senha errada não apaga nada; a espera entre tentativas cresce até 5 min (só em memória: o custo do Argon2id é a defesa contra adivinhação offline). "Esqueci a senha" apaga o banco, as chaves e os tokens. Com o app bloqueado (por tempo sem uso, pelo Windows ou pelo usuário), o banco fica fechado e as sessões e tokens saem da memória. O índice de contas (e-mails) continua no cofre sem a senha. |
| 16 | Conteúdo fora do banco cifrado | Parcialmente mitigada (US10, CA6) | As notificações mostram só a conta por padrão (D5); com "Remetente e assunto", o Windows guarda esse texto sem cifra no próprio banco de notificações. O cache de disco do WebView2 fica com 1 byte (`--disk-cache-size=1`) e os dados de navegação são limpos ao bloquear (D6); falta conferir no app real que nada do e-mail fica na pasta do WebView2. O arquivo de paginação e o de hibernação podem receber memória do app; só o BitLocker os protege, e Privacidade avisa quando o disco não está cifrado. |
| 17 | Computador quântico no futuro | Parcialmente mitigada | Dados guardados: resistem (AES-256). Tráfego gravado hoje: a troca de chaves do TLS não resiste (US11). Backup RSA do DPAPI em domínio: não resiste (ver 15). |
| 12 | Infraestrutura ou ordem legal | Fora do escopo do Epistula | Não há infraestrutura do Epistula. Pedidos legais alcançam os dados no Google. |

**Risco aceito pela D4.** Depois de sair de uma conta, as mensagens salvas dela continuam no disco (cifradas). Sem a senha do Epistula, qualquer pessoa com acesso à sessão do usuário no Windows consegue lê-las pelo Epistula. Desde a US10, o diálogo de sair e Configurações → Privacidade apagam os dados da conta e trocam a chave do banco.

## Premissas

- O keyring do SO protege os itens da sessão do usuário contra outros usuários locais.
- A WebView do sistema (WebView2 no Windows) aplica corretamente `sandbox`, `srcdoc` e CSP.
- O `ammonia` (allowlist sobre o parser html5ever) é a barreira principal contra HTML ativo; o iframe sem scripts é a segunda.
- O SQLCipher 4 com as configurações fixas em `db.rs` (AES-256-CBC, HMAC-SHA512, chave crua sem KDF, porque já vem do gerador do SO) é adequado para cifra em repouso.
- O app não faz requisições de rede para desenhar a tela: fontes e estilos vêm empacotados (sem Google Fonts), e a CSP só permite fontes e estilos locais.
- O escopo OAuth do Google é `https://mail.google.com/ openid email profile`. Ele já cobre o SMTP do envio. `profile` (não sensível) só serve para a foto e o nome do `From`; sem ele, a conta funciona com a inicial e envia só com o endereço.
- O servidor de envio grava a cópia em Enviados (comportamento do Gmail); o app não faz `APPEND` e só sincroniza Enviados depois.
- Testes contra o Gmail real são manuais: não há servidor IMAP com XOAUTH2 em CI. O banco tem testes de integração reais (SQLCipher em diretório temporário).

## Riscos residuais priorizados

1. Releases sem assinatura nem atualização verificada (ameaça 5): antes de distribuir.
2. Verificação do app OAuth pelo Google, com avaliação de segurança, para sair do modo Testing.
3. Auditoria contínua das dependências: o CI atual roda clippy, testes e gitleaks; `cargo deny` roda só localmente.
4. Dados de contas de que o usuário saiu continuam no disco até ele apagar (D4); Privacidade e o diálogo de sair oferecem apagar (US10, CA7).
5. Sem a senha do Epistula (opcional), quem tem acesso ao Windows do usuário tem acesso à chave do banco e aos tokens (ameaça 15).
6. Conteúdo fora do banco: notificações, cache do WebView2, paginação e hibernação (ameaça 16, US10, CA6).
7. Indicadores de autenticidade do remetente (SPF/DKIM/DMARC dos cabeçalhos `Authentication-Results`).

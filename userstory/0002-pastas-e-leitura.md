## User Story 2: Navegar pelas pastas do Gmail e ler mensagens com segurança

**Como** usuário com o Gmail conectado ao Epistula,
**quero** navegar pelas minhas pastas, paginar as mensagens, abrir e ler o conteúdo, e marcar mensagens como lidas, não lidas ou com estrela,
**para que** eu use o Epistula como meu cliente de leitura do dia a dia sem expor meu computador a conteúdo malicioso nem vazar dados em logs.

---

### 1. Contexto

A US1 entregou a conexão OAuth (IMAP + XOAUTH2) e a lista dos 50 cabeçalhos mais recentes da INBOX. Esta story transforma o Epistula num leitor completo de uma conta: pastas, paginação, leitura do corpo e flags.

Ela segue o documento `prompt_backend_email_rust.md`, adaptado à arquitetura que já existe. Pela regra 2 do prompt, as decisões da US1 são preservadas: Tauri 2, backend Rust local, frontend React + TS, design system em `design-system/`, OAuth com PKCE e loopback, tokens só no keyring e na memória do processo Rust.

**Relação com o prompt (o que entra e o que não se aplica agora)**

| Item do prompt | Nesta story |
|---|---|
| Fase 1 §3 — pastas padrão, estados lida/não lida e sinalizada, paginação por cursor, ordenação estável, IDs opacos, idempotência | **Entra** |
| Fase 1 §3 — não decifrar/indexar/regras no servidor | Não se aplica: não há servidor próprio; nada é indexado nem persistido |
| Regra 7 e "Logs, privacidade" — nada de assunto, corpo, endereços ou tokens em logs, com teste de redaction | **Entra** |
| "Testes de segurança" — fuzzing de parsers MIME e entradas externas | **Entra** (testes de propriedade e corpus de ataques) |
| Entregável 2 — documento de arquitetura e modelo de ameaças | **Entra** (`docs/arquitetura-e-ameacas.md`) |
| "Segurança de infraestrutura" — auditoria de dependências | **Entra** (`cargo audit` e `cargo deny`) |
| Fase 1 §1 (contas próprias, hash de senha, sessões), §2 (E2EE), §4 (envio), Postgres, Axum, CSRF/CORS | Fora: dependem da decisão D1 (seção 9) ou de stories próprias |

---

### 2. Regra do design system (obrigatória)

Vale integralmente a seção 2 da US1:

- fonte única em `design-system/`;
- só componentes e tokens do DS;
- wrappers finos em `src/ui/` citando a página do DS;
- lacunas bloqueiam e são registradas na seção 4;
- temas claro e escuro, acessibilidade.

Situação das lacunas da US1: L1 e L2 descartadas; L3–L7 resolvidas. O app não tem mais CSS próprio.

### 3. Mapa de telas para componentes do DS

| Tela / estado | Componentes do DS | Página de referência |
|---|---|---|
| Layout de três painéis (sidebar + lista + leitura) | `.app-shell` (sem `--no-sidebar`) com `.sidebar.app-shell__sidebar`; abaixo de 1280px a sidebar recolhe sozinha, e cada `.nav-item` leva `data-tip` | `espacamento.html` ("Casca do cliente") |
| Lista de pastas | `.sidebar`, `.nav-group`, `.nav-item` com ícone, `[aria-current="page"]`, `.nav-item__count`, `.has-unread` | `sidebar.html` |
| Marcadores do Gmail (pastas personalizadas) | `.nav-heading` "Marcadores" + `.nav-item` com ícone `folder` | `sidebar.html` |
| Paginação | `.toolbar__end` com texto "1–50 de 1.284" + `.btn--subtle.btn--icon.btn--compact` (`chevron-left`/`chevron-right`) | `threads.html` |
| Estrela na linha | `.star-btn.thread__star` interativa (`aria-pressed`) | `threads.html` |
| Ação rápida no hover da linha | `.thread__actions` só com "Marcar como lida/não lida" (`mail-open`/`mail`) | `threads.html` |
| Cabeçalho da leitura | `.reader__head`, `.reader__subject` + ações `.btn--subtle.btn--icon` (estrela, marcar como não lida) | `leitura.html` |
| Remetente e destinatários | `.message__head`, `.avatar--lg`, `.message__from`, `.message__addr`, `.message__to[aria-expanded]` + `.message__recipients` (De/Para/Cc/Data) | `leitura.html` ("Destinatários expandidos") |
| Corpo em texto puro | `.message__body` com parágrafos `<p>` | `leitura.html` |
| Corpo em HTML | `.message__frame` (`<iframe sandbox>`) dentro de `.message__body`, fundo `--color-surface-paper` | `leitura.html` ("Corpo em HTML") |
| Imagens externas bloqueadas | `.section-message` (info) "Imagens externas bloqueadas" + ação "Mostrar imagens" | `mensagens.html` (exemplo idêntico já publicado) |
| Anexos (só metadados) | `.attachments`, `.attachment.attachment--static`, `.attachment__icon--pdf/--img/--doc`, `.attachment__name`, `.attachment__meta` | `anexos.html` ("Somente informação") |
| Carregando o corpo | `.reader[aria-busy]` com `.skeleton--title`, `.skeleton--avatar` e `.skeleton--line` | `carregamento.html` ("Painel de leitura") |
| Falha ao abrir a mensagem | `.section-message--error` + "Tentar de novo" | `mensagens.html` |
| Falha ao mudar flag | `.toast--error` + "Tentar de novo" | `toasts.html` |
| Pasta vazia | `.empty` "Tudo em dia" (Entrada) ou "Nenhuma mensagem nesta pasta" (demais) | `vazios.html` |
| Mensagem removida no servidor | `.section-message--warning` "Esta mensagem não está mais nesta pasta" | `mensagens.html` |

Mapeamento de pastas do Gmail (atributos SPECIAL-USE da RFC 6154, nunca pelo nome, que muda com o idioma):

| Atributo IMAP | Papel | Rótulo | Ícone DS |
|---|---|---|---|
| `INBOX` | `inbox` | Caixa de entrada | `inbox` |
| `\Flagged` | `starred` | Com estrela | `star` |
| `\Sent` | `sent` | Enviados | `send` |
| `\Drafts` | `drafts` | Rascunhos | `file` |
| `\All` | `all` | Todos os e-mails (é o "Arquivo" do Gmail) | `archive` |
| `\Junk` | `spam` | Spam | `spam` |
| `\Trash` | `trash` | Lixeira | `trash` |
| `\Important` | — | Oculta nesta story | — |
| sem atributo | `custom` | Nome do marcador (UTF-7 modificado decodificado) | `folder` |
| `\Noselect` | — | Não selecionável: não é listada | — |

### 4. Lacunas no design system

Todas as lacunas desta story foram resolvidas no DS em 23 out 2026:

| # | Lacuna | Resolução no DS |
|---|---|---|
| L10 | Contêiner para corpo HTML | `.message__frame` e token `--color-surface-paper` (claro nos dois temas), em `leitura.html` |
| L11 | Skeleton do painel de leitura | `.skeleton--title`, `.skeleton--avatar`, `.skeleton--line`, em `carregamento.html` |
| L12 | Anexo sem download | `.attachment--static`, em `anexos.html` |
| L13 | Destinatários expandidos | `.message__recipients` e chevron girado em `.message__to[aria-expanded="true"]`, em `leitura.html` |

A altura do `.message__frame` é definida pelo app depois de medir o conteúdo, e o mínimo é 8rem. Para medir, o iframe precisa de `allow-same-origin`, sempre **sem** `allow-scripts`: sem scripts, a mesma origem não dá ao conteúdo nenhum poder de execução. A alternativa é altura fixa com rolagem interna; decidir na implementação e cobrir no corpus do CA4.

### 5. Critérios de aceite

**CA1. Pastas**

- O comando `list_folders` executa `LIST "" "*"`, classifica as pastas pelos atributos da tabela da seção 3 e devolve `Folder { id, role, name, unread, total }`, com as pastas do sistema na ordem da tabela e os marcadores em ordem alfabética.
- `unread` e `total` vêm de `STATUS (MESSAGES UNSEEN)`, só para as pastas do sistema. Para marcadores, `unread` é carregado só quando a pasta é aberta (o Gmail pode ter centenas de marcadores).
- Nomes de marcadores em UTF-7 modificado (RFC 3501 §5.1.3) são decodificados. O `id` é opaco e o frontend nunca envia nomes IMAP crus.
- A sidebar abre na Caixa de entrada; a pasta atual usa `aria-current="page"`.

**CA2. Paginação por cursor**

- `list_messages({ folderId, cursor? })` devolve `{ items: MessageHeader[], nextCursor?, total }`, com 50 itens por página, do mais novo para o mais antigo por UID.
- O cursor é opaco e codifica `{ pasta, UIDVALIDITY, antes_do_uid }`. A página seguinte usa `UID SEARCH UID 1:<antes_do_uid - 1>` e pega os 50 maiores. Mensagens novas não deslocam páginas já carregadas.
- Se o `UIDVALIDITY` mudou desde a emissão do cursor, devolve `CursorExpired`; o frontend volta à primeira página sem mensagem de erro.
- "Página anterior" usa a pilha de cursores mantida no frontend; o backend não guarda estado de paginação.
- O `MessageHeader` passa a ter `id` opaco (pasta + `UIDVALIDITY` + UID) no lugar do `uid` cru.

**CA3. Leitura da mensagem**

- `get_message({ messageId, allowRemoteImages })` busca `RFC822.SIZE` e, se o tamanho for de até **25 MB**, `BODY.PEEK[]`. Acima disso, devolve só os cabeçalhos e `MessageTooLarge`.
- O backend faz o parse MIME (`mail-parser`) e devolve `MessageView { id, subject, from, to[], cc[], date, flags, body, blockedRemoteImages, attachments[] }`.
- `body` é `{ kind: "html", html }` quando houver `text/html`, senão `{ kind: "text", text }`, senão `{ kind: "none" }`.
- `attachments[]` traz `{ name, mimeType, size }`. Nomes são tratados como texto não confiável e exibidos escapados. Imagens inline (`cid:`) aparecem como anexos nesta story.
- `Bcc` nunca é exibido nem devolvido.

**CA4. HTML seguro (requisito de segurança)**

- O HTML é sanitizado **no Rust** com o crate `ammonia`, por allowlist de tags e atributos, antes de sair do backend. O frontend nunca recebe HTML não sanitizado.
- Removidos sempre:
  - `<script>`, `<style>`, `<link>`, `<meta>`, `<base>`, `<iframe>`, `<frame>`, `<object>`, `<embed>`, `<form>`, `<input>`, `<button>`, `<svg>`, `<math>`;
  - atributos `on*`;
  - URLs `javascript:`, `vbscript:` e `data:` (exceto `data:image/*` em `<img>`);
  - declarações CSS com `url(`, `expression(`, `@import` ou `behavior` dentro de `style`.
- Imagens remotas: com `allowRemoteImages = false`, todo `src`/`srcset` `http(s)` é removido, e `blockedRemoteImages` informa quantas eram. O frontend mostra a section message "Imagens externas bloqueadas" com "Mostrar imagens", que repete a chamada com `true` só para aquela mensagem. Assim nenhum rastreador abre sem ação do usuário.
- O HTML é renderizado num `<iframe sandbox>` **sem** `allow-scripts` e **sem** `allow-same-origin`, via `srcdoc`, com uma meta CSP própria: `default-src 'none'; style-src 'unsafe-inline'; img-src data:` (mais `https:` só quando o usuário liberou imagens).
- Links: abrem **somente** no navegador do sistema, pelo backend (`tauri-plugin-opener`), e só com esquema `http`, `https` ou `mailto`. Nunca navegam a WebView do app. A técnica de interceptação (atributo de alvo + handler de navegação/nova janela do Tauri) é escolhida na implementação, e o teste de aceite cobre clique em link `https`, em `javascript:` (ignorado) e em link relativo (ignorado).
- A CSP da janela principal (`tauri.conf.json`) continua sem `unsafe-eval` e sem origens remotas para scripts.

**CA5. Flags (lida, não lida, estrela)**

- Abrir uma mensagem a marca como lida (`UID STORE +FLAGS (\Seen)`) depois que o corpo carregou com sucesso.
- `set_flags({ messageIds[], seen?, flagged? })` aplica `+FLAGS`/`-FLAGS` por UID e devolve as flags resultantes de cada mensagem. É idempotente: repetir a chamada dá o mesmo estado.
- A UI aplica a mudança na hora (linha, sidebar e leitura) e reverte com toast de erro se o servidor recusar.
- A estrela usa `\Flagged`, que no Gmail é a própria pasta "Com estrela".
- Os contadores da sidebar são atualizados após cada mudança, sem recarregar a lista.

**CA6. Conexão IMAP reutilizada**

- O backend mantém **uma** sessão IMAP autenticada por conta, protegida por mutex, e a reutiliza entre comandos (a US1 abria uma conexão por listagem).
- Em erro de I/O ou `BYE`, reconecta uma vez com o token garantido válido e repete a operação. Persistindo o erro, devolve `ImapConnection`.
- A sessão é encerrada (`LOGOUT`) em `disconnect` e ao fechar o app.
- Nunca mais de 2 conexões simultâneas por conta (o limite do Gmail é 15).

**CA7. Erros novos**

- `CursorExpired`: o frontend reinicia a paginação.
- `MessageNotFound` (UID não existe mais ou `UIDVALIDITY` mudou): "Esta mensagem não está mais nesta pasta".
- `MessageTooLarge`: "Esta mensagem é grande demais para abrir no Epistula (limite de 25 MB)".
- `InvalidId`: id opaco malformado. É erro de programação e não chega à UI como texto técnico.
- Todos seguem o contrato `{ kind, message }` da US1.

**CA8. Privacidade em logs (com teste)**

- Logs registram só: comando, pasta por **papel** (`inbox`, `custom`), contagens, tamanhos, duração e códigos de erro.
- Nunca: assunto, remetente, destinatário, corpo, nome de anexo, nome de marcador, token.
- Um teste instala um `tracing` subscriber que grava em memória, roda o pipeline de parse e sanitização com uma mensagem sintética contendo marcadores únicos (assunto, corpo, endereço, nome de anexo) e falha se algum marcador aparecer na saída.

### 6. Comandos Tauri (contrato)

| Comando | Entrada | Saída |
|---|---|---|
| `list_folders` | — | `Folder[]` |
| `list_messages` | `{ folderId, cursor? }` | `{ items: MessageHeader[], nextCursor?, total }` |
| `get_message` | `{ messageId, allowRemoteImages }` | `MessageView` |
| `set_flags` | `{ messageIds: string[], seen?: bool, flagged?: bool }` | `{ id, flags }[]` |
| `open_external_link` | `{ url }` | `void` (só `http`, `https`, `mailto`) |
| `list_inbox` | — | Removido: substituído por `list_messages` com a pasta `inbox` |

Os comandos da US1 (`get_account`, `connect_account`, `cancel_connect`, `disconnect`) continuam iguais.

### 7. Entregáveis além do código

1. `docs/arquitetura-e-ameacas.md`, curto. Contém:
   - componentes e limites de confiança: WebView, backend Rust, keyring do SO, Google OAuth, Gmail IMAP, conteúdo de e-mail de terceiros;
   - matriz de dados: campo, onde fica, por quanto tempo, quem acessa;
   - as 12 ameaças do prompt, cada uma marcada como mitigada, parcialmente mitigada ou fora do escopo, com destaque para:
     - (7) dispositivo comprometido;
     - (8) provedor externo sem E2EE: o Google lê os e-mails, e isso precisa estar escrito;
     - (11) logs e dumps de memória;
     - (5) cadeia de dependências.
2. `src-tauri/deny.toml` (licenças permitidas e advisories) e o resultado de `cargo audit` e `cargo deny check` anexado ao relatório da story.
3. `git init` do repositório com o primeiro commit da US1. Pré-requisito para revisão e CI; o `.gitignore` já exclui `.env` e `client_secret_*.json`.

### 8. Fora do escopo

- Arquivar, excluir, mover, spam/não spam (ações destrutivas, com "Desfazer"): story seguinte.
- Download e abertura de anexos.
- Conversas agrupadas (threads, `X-GM-THRID`).
- Busca.
- Cache local, modo offline e sincronização (decisão D2).
- Envio, rascunhos e composição.
- E2EE (decisão D3).
- Outlook e múltiplas contas.

### 9. Decisões pendentes (registradas conforme a regra 4 do prompt)

**D1. Que produto é o Epistula?** Bloqueia o roadmap, não esta story.

O prompt descreve o backend de um **serviço de e-mail próprio**: contas próprias, Postgres, Axum, E2EE em que o provedor não lê o conteúdo. O que existe hoje é um **cliente desktop de provedores externos** (Gmail), em que o Google lê tudo por definição.

- **A. Cliente de provedores externos (atual).** O "backend" é o processo Rust local. A privacidade vem de não ter servidor próprio. E2EE só via OpenPGP entre usuários que também o usem. As Fases 1 §1 e §4 do prompt viram integração IMAP/SMTP com o provedor.
- **B. Serviço próprio com E2EE.** Exige servidor (Axum + Postgres), domínio, entrega SMTP com SPF, DKIM e DMARC, antiabuso, criação de contas e um modelo de chaves no cliente. O app desktop vira cliente desse servidor, e o Gmail sai do foco.
- **C. Híbrido.** Contas externas por IMAP (A) e, depois, contas Epistula com E2EE (B), no mesmo app. É o caminho mais longo e com mais superfície de ataque.

Recomendação: seguir com **A** até a leitura, a organização e o envio estarem sólidos, e só então reavaliar B ou C com um documento de arquitetura próprio.

**D2. Cache local.** O prompt exige conteúdo protegido persistido só cifrado.

- Opção 1: sem cache, sempre online (esta story).
- Opção 2: SQLite cifrado (SQLCipher) com a chave no keyring do SO.

Decidir antes da story de busca/offline.

**D3. E2EE para contas externas.** OpenPGP no cliente (rPGP ou Sequoia; avaliar licenças: Sequoia é LGPL), com chaves privadas só no dispositivo. Story própria, depois de D1.

### 10. Testes obrigatórios

- **Unitários:**
  - classificação de pastas pelos atributos;
  - decodificação de UTF-7 modificado;
  - codificação e validação do cursor e do id opaco (inclusive adulterados e truncados);
  - detecção de mudança de `UIDVALIDITY`;
  - escolha de `text/html` vs `text/plain`;
  - montagem de `+FLAGS`/`-FLAGS`;
  - parágrafos de texto puro.
- **Corpus de ataques ao sanitizador**, em tabela de casos:
  - `<script>`, `<img onerror>`, `<a href="javascript:">`, `<svg onload>`;
  - `<math>` com `xlink:href`, `<iframe srcdoc>`, `<meta http-equiv=refresh>`, `<base href>`, `<form action>`;
  - `style="background:url(...)"`, `@import`, `srcset` remoto, `<link rel=stylesheet>`;
  - comentário condicional do Outlook, entidades e tags quebradas.
  - Para cada caso: nenhuma execução e nenhum carregamento remoto possível.
- **Propriedade (proptest):** bytes arbitrários no parse MIME e no sanitizador nunca causam pânico, e a saída do sanitizador nunca contém `<script` nem `javascript:`, ignorando maiúsculas e minúsculas.
- **Redaction:** o teste do CA8.
- **Frontend (vitest):** pilha de cursores (próxima/anterior), atualização otimista de flags com rollback, formatação de destinatários.
- **Manual contra o Gmail real** (conta de teste):
  - pastas e contadores corretos;
  - paginar até a 3ª página e voltar;
  - abrir HTML com imagens e liberar imagens;
  - abrir texto puro;
  - estrela e lida/não lida refletidas no Gmail web;
  - mensagem apagada no Gmail web enquanto aberta no Epistula.

O prompt pede testes de integração reais, não só mocks. Não há servidor IMAP com XOAUTH2 para CI nesta fase, então o teste manual contra o Gmail é obrigatório e o relatório informa essa limitação.

### 11. Arquivos esperados

| Camada | Arquivo | Responsabilidade |
|---|---|---|
| Rust | `src-tauri/src/mailbox.rs` | Pastas, classificação SPECIAL-USE, UTF-7, `STATUS` |
| Rust | `src-tauri/src/message.rs` | Ids e cursores opacos, parse MIME, `MessageView` |
| Rust | `src-tauri/src/sanitize.rs` | Sanitização de HTML e bloqueio de recursos remotos |
| Rust | `src-tauri/src/imap_client.rs` | Sessão reutilizada, reconexão, `UID SEARCH/FETCH/STORE` |
| Rust | `src-tauri/src/commands.rs` | Comandos da seção 6 |
| Rust | `src-tauri/deny.toml` | Política de licenças e advisories |
| TS | `src/features/mailbox/FolderNav.tsx` | Sidebar de pastas |
| TS | `src/features/inbox/InboxList.tsx` | Lista genérica por pasta, paginação, flags |
| TS | `src/features/message/MessageView.tsx` | Leitura, corpo HTML/texto, imagens, anexos |
| TS | `src/ui/Sidebar.tsx`, `src/ui/Attachment.tsx`, `src/ui/MessageFrame.tsx`, `src/ui/Toast.tsx` | Wrappers do DS (`MessageFrame` usa `.message__frame`) |
| Docs | `docs/arquitetura-e-ameacas.md` | Entregável 1 da seção 7 |

### 12. Definição de pronto

1. Todos os CAs verificados no app contra uma conta Gmail de teste.
2. Corpus de ataques, testes de propriedade, redaction e unitários passando; resultados reais informados.
3. `cargo clippy`, `tsc --noEmit`, `cargo audit` e `cargo deny check` sem erros, ou com exceções justificadas por escrito.
4. Nenhum `unwrap()`/`expect()` em caminhos de rede, parse ou dados do servidor.
5. `docs/arquitetura-e-ameacas.md` revisado.
6. Telas usam as classes de L10–L13 publicadas no DS.
7. Repositório git inicializado; nenhum segredo commitado.
8. Decisão D1 registrada pelo dono do produto, mesmo que seja "seguir com A".

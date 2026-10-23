## User Story 1: Conectar o Gmail via OAuth (IMAP + XOAUTH2) e listar a caixa de entrada

**Como** usuário do Gmail que não quer criar uma conta nova,
**quero** conectar minha conta existente ao Epistula autorizando o acesso na página do Google,
**para que** eu acesse minha caixa de entrada no cliente desktop sem digitar senha nem configurar servidores.

---

### 1. Contexto

Primeiro fluxo vertical do aplicativo: uma fatia fina, compilável e testável. O usuário abre o app, clica em "Conectar com Google", autoriza no navegador do sistema e vê os 50 cabeçalhos mais recentes da INBOX.

**Escopo desta story: somente Gmail.** O backend já é parametrizado por provedor, mas o Outlook fica para a próxima story.

**Por que OAuth e não senha:** o Gmail não aceita a senha da conta no IMAP. A senha de app exige verificação em duas etapas e não funciona em contas Workspace bloqueadas pelo administrador nem com Proteção Avançada. O OAuth aqui não é "login social": ele só emite o token que o IMAP aceita via SASL XOAUTH2. O app continua falando IMAP com `imap.gmail.com:993`.

**Stack obrigatória**

- Tauri 2 (shell desktop).
- Rust no backend: OAuth, keyring, IMAP. Toda credencial vive no processo Rust.
- TypeScript + React + Vite no frontend (WebView). O frontend só renderiza e chama comandos Tauri tipados.
- Design system Epistula, na pasta `design-system/` do repositório (documentação: `design-system/index.html`).

**Decisão arquitetural crítica:** o OAuth roda no backend Rust com **PKCE S256 + loopback redirect (RFC 8252)**. O WebView nunca recebe `access_token`, `refresh_token` nem `code`. O frontend recebe apenas dados de exibição (e-mail da conta, cabeçalhos).

**Pré-requisito no Google Cloud (uma vez, pelo dono do projeto)**

1. Projeto dedicado (ex.: `epistula-desktop`).
2. Google Auth Platform → Branding: nome "Epistula" e e-mail de suporte.
3. Audience: tipo **External**, status **Testing**, com as contas de teste cadastradas em "Test users".
4. Data access: escopos `https://mail.google.com/`, `openid`, `email`.
5. Clients → **Desktop app**. O client ID e o client secret vão para o `.env` local (`EPISTULA_GOOGLE_CLIENT_ID`, `EPISTULA_GOOGLE_CLIENT_SECRET`), que não é versionado. Em apps desktop o Google não trata o secret como confidencial; a segurança vem do PKCE.

O gcloud cobre só o passo 1. Tela de consentimento e client OAuth de contas sem organização não têm API pública: a API IAP OAuth Admin foi desligada em março de 2026.

---

### 2. Regra do design system (obrigatória)

1. **Fonte única.** O frontend importa `design-system/tokens.css` e `design-system/components.css` diretamente da pasta do design system. É proibido copiar, sobrescrever ou "ajustar" essas folhas dentro de `src/`.
2. **Só componentes do DS.** Todo elemento visual é uma classe documentada em `components.css` (`.btn`, `.thread`, `.empty`...) ou uma composição dessas classes, igual a um exemplo publicado no DS.
3. **Só tokens.** Qualquer CSS próprio do app (apenas layout: grid, posicionamento de painéis) usa somente custom properties de `tokens.css` (`--space-*`, `--layout-*`, `--color-*`). Nenhum hex, px solto de cor/raio/sombra, nem fonte nova.
4. **Componente React = wrapper fino.** Cada componente em `src/ui/` renderiza o markup de uma página do DS e cita essa página num comentário de cabeçalho. Nada de estilo inline além de valores vindos de tokens.
5. **Lacunas bloqueiam.** Se uma tela precisar de algo que não existe no DS, o desenvolvedor **não cria o componente no app**: registra a lacuna na seção 4 desta story e pede a criação ou atualização do exemplo no DS. Enquanto isso, usa o fallback indicado na tabela de lacunas.
6. **Temas e acessibilidade.** O app herda os temas claro/escuro de `tokens.css` (segue o sistema) e as regras de `acessibilidade.html`: foco visível, `aria-label` + tooltip em todo botão de ícone, estado comunicado por forma e peso além da cor.

### 3. Mapa de telas para componentes do DS

| Tela / estado | Componentes do DS | Página de referência |
|---|---|---|
| Casca do app | `.app-shell.app-shell--no-sidebar` (`__header`, `__banner`, `__center`, `__list`, `__list-body`, `__reader`) + `.appbar`, `.appbar__brand` com `.wordmark.wordmark--sm`, `.appbar__actions` | `espacamento.html`, `header.html` |
| Conectar conta (inicial) | `.empty` (`__title`, `__desc`, `__actions`) + `.btn.btn--primary` com ícone `i-mail` | `vazios.html`, `botoes.html` |
| Aguardando autorização no navegador | `.empty` + `.spinner.spinner--lg.empty__spinner` + `.btn.btn--subtle` "Cancelar" | `vazios.html` ("Aguardando ação fora do app") |
| OAuth cancelado, negado ou expirado | `.section-message--warning` com ação "Tentar de novo" | `mensagens.html` |
| Falha ao conectar (configuração, rede, token) | `.section-message--error` com ação "Tentar de novo" | `mensagens.html` |
| Sessão expirada / acesso revogado | `.banner--error` acima da tela de conexão | `mensagens.html` |
| Indicador "Conectado como ..." | `.appbar__account` (`.avatar--sm` + `.appbar__account-text` + `.appbar__account-email`) dentro de `.appbar__actions` | `header.html` ("Conta conectada") |
| Carregando a INBOX | `.thread-list` com linhas `.thread` de `.skeleton` | `carregamento.html` |
| Barra da lista | `.toolbar` + `.btn--subtle.btn--icon` "Atualizar" + `.toolbar__end` (contagem) | `threads.html` |
| Lista de mensagens | `.thread-list.thread-list--headers` / `.thread` (`__star`, `__from`, `__subject`, `__meta`), `.is-unread`, `.is-open` | `threads.html` ("Só cabeçalho") |
| INBOX vazia | `.empty` "Tudo em dia" | `vazios.html` |
| Falha IMAP ao listar | `.empty` (variante erro) com `.btn--primary` "Tentar de novo" | `vazios.html` |
| Painel de detalhe | `.reader`, `.reader__subject`, `.message__head`, `.avatar--lg`, `.message__from`, `.message__time` | `leitura.html` |
| Corpo ainda não carregado | `.section-message` (info) | `mensagens.html` |
| Tooltips dos botões de ícone | `.tooltip.tooltip--floating` via `data-tip` | `menus.html` |

Mapeamento de flags IMAP para estados do DS:

- Sem `\Seen` → `.thread.is-unread` (peso 600, ponto azul, data em azul).
- `\Flagged` → `.star-btn[aria-pressed="true"]` somente leitura (sem ação nesta story).
- Mensagem selecionada → `.thread.is-open`.
- Checkbox e ações rápidas de hover da linha **não** são renderizados nesta story.

### 4. Lacunas no design system

Situação em 23 out 2026:

| # | Lacuna | Situação | No app |
|---|---|---|---|
| L1 | Ícone de marca Google | **Descartada**: o DS não terá ícones de marca | Botão "Conectar com Google" com ícone `i-mail` (definitivo) |
| L2 | Sprite de ícones embutido em `docs.js` | **Descartada**: não haverá `icons.svg` | Plugin do Vite extrai o sprite de `docs.js` em tempo de build (definitivo); o build falha se o formato mudar |
| L3 | Wordmark/glyph da marca só em `docs.css` | **Resolvida**: `.wordmark`, `.wordmark__glyph` e `.wordmark--sm` movidos para `components.css` (`header.html`, guia "Marca") | `.appbar__brand` com `.wordmark.wordmark--sm` |
| L4 | Classe de layout do cliente (`.app-shell`) | **Resolvida**: `.app-shell` e variações (`--no-sidebar`, `--embedded`, `__center`, `.has-reader`, `.sidebar--collapsed`) em `components.css` (`espacamento.html`, "Casca do cliente") | `.app-shell.app-shell--no-sidebar`; `src/styles/layout.css` foi removido |
| L5 | Conta conectada no header | **Resolvida**: `.appbar__account`, `.appbar__account-text`, `.appbar__account-email` (`header.html`, "Conta conectada") | Usa as classes oficiais |
| L6 | Estado "aguardando ação externa" | **Resolvida**: `.empty` + `.spinner--lg.empty__spinner` (`vazios.html`, "Aguardando ação fora do app") | Usa as classes oficiais |
| L7 | Linha só com cabeçalho | **Resolvida**: `.thread-list--headers` (`threads.html`, "Só cabeçalho") | Usa a classe oficial |

### 5. Critérios de aceite

**CA1. Tela de conexão**

- Sem conta salva, o app mostra "Conecte seu Gmail" (seção 3) com o botão "Conectar com Google" (`.btn--primary`).
- Clique chama `connect_account({ provider: "google" })` e a tela passa para "Continue no navegador", com spinner e "Cancelar".
- "Cancelar" chama `cancel_connect` e volta à tela inicial sem mensagem de erro.
- Nenhum campo de senha ou servidor é exibido.

**CA2. OAuth no backend Rust**

- O backend abre um listener HTTP em `127.0.0.1:0` (porta efêmera atribuída pelo SO) e monta `redirect_uri = http://127.0.0.1:<porta>`. Clients "Desktop app" do Google aceitam qualquer porta de loopback.
- Gera `code_verifier`/`code_challenge` (PKCE S256) e um `state` aleatório de uso único.
- Abre o navegador do sistema em `https://accounts.google.com/o/oauth2/v2/auth` com escopos `https://mail.google.com/ openid email`, `access_type=offline` e `prompt=consent` (garante `refresh_token`).
- O listener aceita uma única requisição em `/` (outras rotas, como `/favicon.ico`, recebem 404), valida `state`, responde "Pode fechar esta aba" e encerra.
- Troca o `code` por `access_token` + `refresh_token` + `expires_in` + `id_token` em `https://oauth2.googleapis.com/token`.
- O e-mail da conta vem da claim `email` do `id_token`. Ele é obrigatório: é o `user` do SASL XOAUTH2 e o texto de "Conectado como".
- Timeout de 5 minutos sem callback encerra o fluxo com `AuthTimeout`.
- `client_id` e `client_secret` vêm de `EPISTULA_GOOGLE_CLIENT_ID` e `EPISTULA_GOOGLE_CLIENT_SECRET`: em dev, do `.env` na raiz; em build, embutidos no binário via variável de ambiente. Ausentes → `Config`.

**CA3. Armazenamento seguro**

- Persistido no **keyring do SO** (crate `keyring`), serviço `epistula`: entrada `account` com `{ provider, email, parts }` e entradas `secret-0..N` com o `refresh_token`, dividido em partes de até 1.000 caracteres (o Windows limita cada item a 2.560 bytes em UTF-16).
- O `access_token` e o `expires_at` ficam **somente em memória** no processo Rust; na abertura do app o access token é renovado pelo refresh token.
- Nenhum token é gravado em arquivo, `localStorage`, SQLite ou log.
- Se o Google devolver um novo `refresh_token` na renovação, ele substitui o antigo no keyring.

**CA4. IMAP e cabeçalhos**

- Antes de qualquer operação IMAP, o backend garante um access token válido (renova se faltar menos de 60 s para expirar).
- Autentica em `imap.gmail.com:993` (TLS) com **SASL XOAUTH2** (`user=<email>\x01auth=Bearer <token>\x01\x01`).
- Executa `LIST "" "*"` (registrado só como contagem em log de debug) e `SELECT INBOX`.
- Busca os **50 mais recentes**: com `EXISTS = N`, `FETCH max(1, N-49):N (UID FLAGS INTERNALDATE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE)])`. Com `N = 0`, retorna lista vazia sem `FETCH`.
- Cabeçalhos são decodificados (MIME encoded-words RFC 2047, charset) antes de chegar ao frontend. Sem `Date`, usa `INTERNALDATE`.
- Retorna ordenado do mais novo para o mais antigo: `MessageHeader { uid, fromName, fromAddress, subject, date (ISO 8601), flags: { seen, flagged } }`.
- `PEEK` garante que listar não marca nada como lido.
- Conexão e login com timeout de 15 s; operações seguintes, 30 s.

**CA5. Exibição**

- Após autorizar, a tela de conexão é substituída pela casca do app (seção 3): header com "Conectado como `email`", lista à esquerda (`--layout-list-width`) e painel de leitura à direita.
- Durante o carregamento, a lista mostra linhas skeleton.
- Cada linha mostra remetente (nome, ou endereço se não houver nome), assunto (ou "(sem assunto)") e data: hora (`10:42`) se for hoje, `20 out` se for do ano corrente, `20 out 2025` caso contrário.
- Clique ou <kbd>Enter</kbd> numa linha abre o painel de detalhe com assunto, remetente, endereço, data completa e o aviso de corpo não carregado. <kbd>J</kbd>/<kbd>K</kbd> movem a seleção.
- "Atualizar" na toolbar recarrega a INBOX.

**CA6. Erros**

- Comandos retornam erro tipado (`{ kind, message }`), com `kind` em: `AuthCancelled`, `AuthTimeout`, `AuthFailed`, `TokenExpired`, `ImapConnection`, `ImapAuth`, `Keyring`, `Config`. `message` é texto amigável em português, sem detalhes internos.
- Usuário nega o consentimento (`error=access_denied`) → `AuthCancelled` → section message de aviso + "Tentar de novo".
- Refresh recusado (`invalid_grant`: token revogado ou, em modo Testing, expirado após 7 dias) → o backend apaga a conta do keyring e devolve `TokenExpired`; o app volta à tela de conexão com o banner de sessão expirada.
- O IMAP recusa o token → o backend força um refresh e tenta uma vez; se recusar de novo, `ImapAuth`, e o app volta à tela de conexão com banner.
- Falha de rede ao listar → empty state de erro com "Tentar de novo"; a conta continua salva.
- O detalhe técnico vai só para o log, sem token, `code`, `state` nem conteúdo de mensagem.

### 6. Comandos Tauri (contrato)

| Comando | Entrada | Saída |
|---|---|---|
| `get_account` | — | `Account \| null` (`{ provider, email }`), lido do keyring |
| `connect_account` | `{ provider: "google" }` | `Account` |
| `cancel_connect` | — | `void` |
| `list_inbox` | — | `MessageHeader[]` |
| `disconnect` | — | `void` (apaga o keyring) |

### 7. Fora do escopo

- Outlook (story seguinte; o backend já tem endpoints e escopos da Microsoft).
- Login por senha de app.
- Publicação do app OAuth: verificação do Google para o escopo restrito `https://mail.google.com/`, que inclui avaliação de segurança. Nesta fase o app fica em modo Testing.
- Envio de e-mail (SMTP).
- Corpo das mensagens, anexos, marcar como lida, estrela, arquivar.
- Múltiplas contas simultâneas.
- Pastas além de `INBOX` (a sidebar não é renderizada).
- Sincronização em segundo plano/IDLE.
- E2EE.

### 8. Dependências e riscos

**Crates Rust:** `tauri` 2, `tauri-plugin-opener` (abrir o navegador), `reqwest` (endpoint de token), `sha2` + `base64` + `rand` (PKCE e `state`), `keyring` 3 (features nativas por SO), `async-imap` + `async-native-tls` sobre `tokio`, `mail-parser` (decodificação de cabeçalhos), `serde`, `thiserror`, `tracing`, `dotenvy` (só em dev).

**Riscos**

- Modo Testing: só contas cadastradas em "Test users" conseguem autorizar (até 100), a tela mostra o aviso "app não verificado" e o `refresh_token` expira em 7 dias. Tratado pelo fluxo de `TokenExpired`.
- Produção exige verificação do Google para o escopo restrito `https://mail.google.com/`, com avaliação de segurança anual (CASA). Precisa de story própria antes de distribuir o app.
- Sem `refresh_token`: conferir `access_type=offline` + `prompt=consent`.
- O Gmail limita conexões IMAP simultâneas por conta (15). Cada listagem abre e fecha uma conexão.
- Nenhuma lacuna do design system em aberto.

### 9. Arquivos

| Camada | Arquivo | Responsabilidade |
|---|---|---|
| Rust | `src-tauri/src/oauth.rs` | Loopback, PKCE, troca e refresh de token, claims do `id_token` |
| Rust | `src-tauri/src/provider.rs` | Endpoints, escopos, client OAuth e host IMAP por provedor |
| Rust | `src-tauri/src/keyring_store.rs` | Conta e refresh token no keyring, com divisão em partes |
| Rust | `src-tauri/src/session.rs` | Access token em memória e "garantir token válido" |
| Rust | `src-tauri/src/imap_client.rs` | TLS, XOAUTH2, `SELECT`, `FETCH` e parse dos cabeçalhos |
| Rust | `src-tauri/src/error.rs` | `AppError` tipado e serializável |
| Rust | `src-tauri/src/commands.rs` | Comandos Tauri da seção 6 |
| TS | `src/lib/tauri.ts` | Wrappers tipados de `invoke` e tipos compartilhados |
| TS | `src/ui/*.tsx` | Wrappers finos dos componentes do DS (Button, Icon, EmptyState, Thread, Reader...) |
| TS | `src/features/account/ConnectAccount.tsx` | Tela de conexão e espera do navegador |
| TS | `src/features/inbox/InboxList.tsx` | Toolbar, skeleton, lista e vazio |
| TS | `src/features/inbox/MessageDetail.tsx` | Painel de detalhe |

### 10. Definição de pronto

1. Projeto no Google Cloud configurado (seção 1) e `.env` local preenchido.
2. "Conectar com Google" abre o navegador do sistema, o usuário de teste autoriza e o app recebe o callback.
3. A conta fica no keyring (verificável no gerenciador de credenciais do SO, serviço `epistula`) e nenhum token aparece em arquivo ou log.
4. Os 50 cabeçalhos mais recentes da INBOX aparecem em menos de 5 s após a autorização.
5. Fechar e reabrir o app vai direto à INBOX (refresh automático a partir do keyring).
6. Cancelar no app, negar consentimento no navegador e derrubar a rede mostram os estados de erro da seção 3.
7. Remover o acesso do Epistula em `myaccount.google.com/permissions` e clicar em "Atualizar" volta à tela de conexão com o banner.
8. Revisão de UI: nenhuma classe, cor, raio, sombra ou fonte fora de `tokens.css`/`components.css`; cada componente em `src/ui/` cita sua página do DS.
9. Nenhum fallback local: L1 e L2 foram descartadas; L3–L7 usam as classes oficiais do DS.
10. `cargo clippy` e `tsc --noEmit` sem erros; testes unitários de PKCE/`state`, callback de loopback, claims do `id_token`, SASL XOAUTH2, intervalo de `FETCH`, decodificação de cabeçalhos, divisão do refresh token e formatação de data.
11. Teste manual no SO principal de desenvolvimento (Windows) e, quando possível, em macOS e Linux.

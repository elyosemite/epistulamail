## User Story 3: Banco local, múltiplas contas e saída de conta

**Como** usuário que tem várias caixas de e-mail (por exemplo, João, com um Gmail pessoal, um do trabalho e um da família),
**quero** conectar todas elas no Epistula, trocar de uma para outra com um clique e ver minhas mensagens mesmo quando a rede estiver lenta ou fora,
**para que** o Epistula seja o único lugar onde eu leio todos os meus e-mails, sem confundir uma conta com outra e podendo sair de qualquer uma quando quiser sem perder o que já estava salvo.

---

### 1. Contexto

As US1 e US2 entregaram uma conta Gmail por instalação, sempre online: cada lista e cada mensagem vem do IMAP na hora, e nada é gravado em disco.

Esta story traz quatro mudanças:

1. **Banco local SQLite** com as mensagens, pastas, marcadores e dados de cada conta. A lista abre do banco e a sincronização com o servidor acontece em segundo plano.
2. **Várias contas ao mesmo tempo.** O modelo de dados já nasce preparado para Gmail, Outlook, Yahoo, iCloud Mail, Zoho Mail, GMX, AOL Mail e domínio próprio (IMAP genérico). Nesta story só o Gmail é conectável; os demais aparecem como "Em breve".
3. **Coluna de contas** à esquerda da sidebar, para trocar de conta e reconhecer cada uma pelo nome e pela cor que o usuário escolheu ("Trabalho", "Família", "Pessoal").
4. **Sair de uma conta**: o acesso ao provedor é encerrado e a senha de acesso apagada, mas as mensagens salvas continuam no banco local, cifradas, e voltam a sincronizar se o usuário entrar de novo.

Ela segue `prompt_backend_email_rust.md` no que se aplica a um cliente local (decisão D1, opção A, ainda pendente de confirmação). Em especial: conteúdo persistido só cifrado, nada de conteúdo em logs, migrações versionadas, restrições e índices no banco, testes reais de integração com o banco.

**Relação com o prompt**

| Item do prompt | Nesta story |
|---|---|
| "Conteúdo protegido persistido só cifrado" | **Entra**: banco cifrado com SQLCipher (decisão D2, seção 9) |
| Regras de banco: chaves estrangeiras, índices, unicidade, migrações versionadas | **Entra** |
| Regra 7 e "Logs, privacidade" | **Entra**, estendida: logs identificam a conta só pelo id interno, nunca pelo endereço |
| "Revogação de sessões" | **Entra**: sair revoga o token no Google e apaga o segredo do keyring |
| "Testes de integração reais" | **Entra** para o banco (SQLite real em diretório temporário). IMAP continua manual |
| Busca local (`search`) | Fora: story própria, que usará este banco (FTS5) |
| E2EE, envio, contas próprias | Fora: D1 e D3 |

---

### 2. Regra do design system (obrigatória)

Vale integralmente a seção 2 da US1: só componentes e tokens de `design-system/`, wrappers finos em `src/ui/` citando a página do DS, lacunas bloqueiam e são registradas na seção 4, temas claro e escuro, acessibilidade.

### 3. Mapa de telas para componentes do DS

| Tela / estado | Componentes do DS | Página de referência |
|---|---|---|
| Coluna de contas | **Lacuna L15** (`.account-rail`) | `espacamento.html` |
| Conta na coluna (avatar na cor da conta, marcador de conta ativa, contador de não lidas) | **Lacuna L15** sobre `.avatar` + cores `--purple/--teal/--green/--orange/--magenta` + `.badge` | `identidade.html`, `etiquetas.html` |
| Nome e e-mail ao passar o mouse | `.tooltip--floating` | `menus.html` |
| Menu da conta (renomear, mudar cor, mover, sair) | `.menu`, `.menu-item`, `.menu-item--danger` para "Sair desta conta" | `menus.html` |
| Conta ativa no topo | `.appbar__account` com o nome da conta além do e-mail | `header.html` |
| Adicionar conta: escolha do provedor | **Lacuna L16** (grade de provedores, com estado "Em breve") sobre `.choice` + `.lozenge` | `selecao.html` |
| Nomear a conta e escolher a cor | `.modal`, `.field`, `.input`, sugestões em `.chip` ("Pessoal", "Trabalho", "Família"), **Lacuna L17** (seletor de cor) | `modais.html`, `campos.html` |
| Confirmar saída da conta | `.modal` com `.btn--warning` "Sair" e `.btn--subtle` "Cancelar" | `modais.html` |
| Sem conexão, mostrando o que está salvo | `.banner--warning` "Sem conexão. Mostrando mensagens salvas neste computador." | `header.html` |
| Sincronizando | `.spinner--sm` na toolbar da lista, com rótulo para leitor de tela | `carregamento.html` |
| Conta que precisa reconectar (token revogado ou expirado) | `.dot` de alerta no item da coluna + `.section-message--warning` "Reconecte esta conta" na lista | `mensagens.html` |
| Conta de que o usuário saiu | Item da coluna esmaecido (**Lacuna L15**, estado `--signed-out`) + `.section-message` "Você saiu desta conta" com "Entrar de novo" | `mensagens.html` |
| Nenhuma conta | `.app-shell--no-sidebar` + tela de conexão da US1 | `espacamento.html` |

### 4. Lacunas no design system

| # | Lacuna | Proposta para o DS |
|---|---|---|
| L15 | Coluna vertical de contas | `.account-rail` (largura fixa ~64px, rolagem própria, botão "+" no fim), `.account-rail__item` com `[aria-current="true"]` (barra lateral de destaque), contador sobre o avatar, estado de alerta e estado `--signed-out` (esmaecido). Nova área `rail` no `.app-shell`, à esquerda da sidebar, visível em todas as larguras acima de 960px. Abaixo de 960px, a coluna vira um seletor no topo da sidebar. Exemplo em `espacamento.html` e guia em `sidebar.html` |
| L16 | Grade de provedores | Cartões de escolha com ícone genérico (`i-mail`, sem marcas, conforme L1), nome do provedor e `.lozenge` "Em breve" desabilitado. Exemplo em `selecao.html` |
| L17 | Seletor de cor da conta | Grupo de rádios com amostras das 6 cores de avatar (azul padrão + 5 variações), foco visível e nome da cor para leitor de tela. Exemplo em `selecao.html` |

Resolvidas no DS em 23 out 2026, depois de aprovadas num rascunho:

- **L15:** `.account-rail` (`__list`, `__item`, `__name`, `__badge`, `__alert`, `__item--signed-out`, `__sep`, `__add`), `.app-shell--with-rail` com a área `__rail`, `.account-switcher` para telas abaixo de 960px e `.appbar__account-name`. Documentado em `sidebar.html` e `espacamento.html`.
- **L16:** `.provider-grid` e `.provider-card` (`__icon`, `__text`, `__name`, `__desc`), em `selecao.html`.
- **L17:** `.color-picker` e `.color-swatch` (`__check`), em `selecao.html`, mais `.chip-group` para as sugestões de nome.
- Também: `.modal--warning` para ações reversíveis (sair da conta), em `modais.html`, e os ícones `log-out`, `log-in`, `arrow-up`, `arrow-down`, `droplet` e `globe` no sprite.

### 5. Critérios de aceite

**CA1. Banco local cifrado**

- Um único arquivo `epistula.db` em `app_data_dir` do Tauri (no Windows, `%APPDATA%\com.epistula.app\`).
- SQLite 3 com SQLCipher 4 via `rusqlite` (feature `bundled-sqlcipher-vendored-openssl`), sem dependência de DLL do sistema. Pré-requisito de build no Windows: Strawberry Perl (o OpenSSL embutido não configura com o Perl do Git Bash).
- O build do OpenSSL embutido no Windows usa `no-asm`. Antes de fechar a story, o benchmark da D2 é repetido com o binário real; se listar uma página passar de 20 ms, ativar a aceleração por assembly (NASM) ou reavaliar.
- A chave do banco tem 32 bytes do gerador do SO (`OsRng`), gerada no primeiro uso e guardada no keyring (serviço `epistula`, entrada `db-key`). Nunca é gravada em disco nem em log.
- Ao abrir: `PRAGMA key`, depois `foreign_keys = ON`, `journal_mode = WAL`, `secure_delete = ON`, `temp_store = MEMORY`. Uma consulta de verificação falha de forma explícita se a chave estiver errada.
- Se a chave sumiu do keyring ou não abre o banco: o arquivo é renomeado para `epistula.db.unreadable-<data>`, um banco novo é criado e as contas pedem reconexão. O banco é um cache do servidor, então nada se perde no provedor. Isso é registrado em log só como código de erro.
- Migrações versionadas em `src-tauri/migrations/NNNN_nome.sql`, aplicadas em transação na abertura e controladas por `PRAGMA user_version`. Uma migração que falha não deixa o banco em versão intermediária.
- Todo acesso ao banco roda fora da thread da UI (`spawn_blocking` ou pool dedicado) e usa só consultas parametrizadas.

**CA2. Modelo de dados**

| Tabela | Conteúdo principal | Restrições |
|---|---|---|
| `account` | `id` (UUID), `provider`, `email`, `display_name` (nome dado pelo usuário), `color`, `position`, `status` (`active`, `needs_reauth`, `signed_out`), `created_at`, `last_sync_at` | `UNIQUE(provider, email)`, `email` normalizado em minúsculas |
| `provider_config` | Por conta: `auth_kind` (`oauth2`, `password`), `imap_host`, `imap_port`, `imap_security`, `smtp_host`, `smtp_port`, `smtp_security` | FK para `account` com `ON DELETE CASCADE`. Preenchido pelo catálogo para provedores conhecidos e pelo usuário para domínio próprio |
| `folder` | `account_id`, `imap_name`, `role`, `display_name`, `uidvalidity`, `uidnext`, `highestmodseq`, `total`, `unread`, `last_sync_at` | `UNIQUE(account_id, imap_name)`, FK com cascade |
| `message` | `account_id`, `provider_msg_id` (no Gmail, `X-GM-MSGID`), `message_id_header`, `subject`, `from_name`, `from_address`, `to_json`, `cc_json`, `date`, `size`, `seen`, `flagged`, `has_attachments`, `body_state` (`none`, `cached`, `too_large`) | `UNIQUE(account_id, provider_msg_id)`, FK com cascade. Índice por `(account_id, date DESC)` |
| `message_folder` | `message_id`, `folder_id`, `uid` | `UNIQUE(folder_id, uid)`. É a ligação mensagem ↔ pasta: no Gmail a mesma mensagem aparece em várias pastas (marcadores), e é guardada uma vez só |
| `message_body` | `message_id`, `raw` (RFC 822 completo, até 25 MB), `fetched_at` | FK com cascade. O HTML é sanitizado a cada leitura, nunca guardado já sanitizado, para que uma correção do sanitizador valha para mensagens antigas |
| `attachment` | `message_id`, `name`, `mime_type`, `size` (só metadados, como na US2) | FK com cascade |
| `label` | Marcadores/tags por conta: `account_id`, `name`, `folder_id` opcional, `color` | `UNIQUE(account_id, name)`. No Gmail, cada marcador é também uma pasta IMAP; a tabela existe para provedores com tags (Outlook categories, keywords IMAP) |
| `message_label` | `message_id`, `label_id` | Chave composta |
| `account_setting` | `account_id`, `key`, `value` (ex.: última pasta aberta, imagens remotas sempre permitidas para um remetente no futuro) | `PRIMARY KEY(account_id, key)` |

- Os ids opacos da US2 (`m1.`, `c1.`, `f1.`) passam a incluir o `account_id`. Um id de uma conta nunca é aceito num comando de outra conta (`InvalidId`).
- Refresh tokens e senhas **não** ficam no banco: continuam no keyring, agora por conta (CA6).

**CA3. Sincronização e leitura a partir do banco**

- `list_messages` responde do banco. Se a pasta nunca foi sincronizada, sincroniza primeiro e depois responde.
- Sincronização por pasta, em segundo plano, ao abrir a pasta, ao clicar em "Atualizar" e a cada 5 minutos para a Caixa de entrada de cada conta:
  - **novas mensagens:** UIDs a partir do `uidnext` salvo, só cabeçalhos (`ENVELOPE`, `FLAGS`, `RFC822.SIZE`, `X-GM-MSGID`, `BODYSTRUCTURE` para saber se há anexos);
  - **flags alteradas:** com `CONDSTORE` (o Gmail suporta), `FETCH 1:* (FLAGS) (CHANGEDSINCE <highestmodseq>)`;
  - **removidas:** `UID SEARCH` da janela sincronizada comparado com o banco; o que sumiu sai de `message_folder`, e a mensagem sai do banco quando não está em mais nenhuma pasta;
  - **`UIDVALIDITY` mudou:** apaga as ligações daquela pasta e sincroniza de novo.
- Primeira sincronização de uma pasta: as 2.000 mensagens mais recentes (cabeçalhos, flags e metadados de anexos). Mensagens mais antigas são buscadas do servidor quando o usuário pagina ou abre e ficam salvas. Nada salvo é apagado por idade ou tamanho.
- O corpo é baixado ao abrir a mensagem e fica salvo. Abrir de novo, inclusive sem rede, não acessa o servidor.
- Quando a sincronização termina, o frontend recebe um evento Tauri `mailbox-updated { accountId, folderId }` e recarrega a página atual do banco, mantendo a seleção e a rolagem.
- Mudanças de flag (US2, CA5) são aplicadas no banco junto com a UI. Se o servidor recusar, banco e UI voltam ao estado anterior.
- Sem rede: listas e mensagens já salvas abrem normalmente, com o banner "Sem conexão". Ações que exigem servidor (abrir mensagem não salva, mudar flag) mostram o erro de conexão da US2. Fila de ações offline fica fora desta story.
- No máximo 2 conexões IMAP por conta e 1 sincronização por pasta ao mesmo tempo.

**CA4. Várias contas e provedores**

- Catálogo de provedores no Rust (`providers.rs`): Gmail, Outlook, Yahoo, iCloud Mail, Zoho Mail, GMX, AOL Mail e "Outro (domínio próprio)", cada um com nome, servidores IMAP e SMTP padrão, tipo de autenticação e `available: bool`. Nesta story só o Gmail tem `available = true`.
- Tela "Adicionar conta" com a grade de provedores (L16). Os indisponíveis aparecem desabilitados com "Em breve" e não fazem nada ao clicar.
- Conectar uma conta Gmail que já está conectada não cria outra: mostra "Esta conta já está no Epistula" e seleciona a existente (`AccountAlreadyConnected`).
- Depois do OAuth, um modal pede o nome da conta, com sugestões "Pessoal", "Trabalho" e "Família", e a cor (L17). Os padrões são o nome antes do `@` e a próxima cor ainda não usada. O nome tem de 1 a 32 caracteres e não precisa ser único, mas a cor sugerida evita repetição.
- Não há limite fixo de contas. A interface deve funcionar bem com até 10.
- O backend mantém uma sessão IMAP e um access token **por conta**. Os comandos da US2 recebem `accountId`, ou o derivam do id opaco.

**CA5. Coluna de contas**

- Sempre visível à esquerda da sidebar quando há pelo menos uma conta, mesmo com só uma (é onde fica o "+").
- Cada conta aparece como avatar na cor escolhida, com as iniciais do **nome da conta** (não do e-mail, porque as três contas do João têm o mesmo domínio). Na conta ativa há o marcador de seleção; nas demais, o contador de não lidas da Caixa de entrada (acima de 99, "99+").
- O tooltip mostra nome e e-mail: "Trabalho · joao@empresa.com".
- O topo do app (`.appbar__account`) mostra o nome e o e-mail da conta ativa, para não haver dúvida de qual caixa está aberta.
- Trocar de conta:
  - por clique ou por `Ctrl+1` a `Ctrl+9`, na ordem da coluna;
  - abre em menos de 100 ms, do banco, na última pasta usada naquela conta (`account_setting`);
  - não perde o estado das outras contas: voltar para uma conta restaura a pasta e a página onde o usuário estava.
- Menu da conta (clique com o botão direito ou tecla `Menu`/`Shift+F10` no item): "Renomear", "Mudar cor", "Mover para cima", "Mover para baixo" e "Sair desta conta" (ou "Entrar de novo", se o usuário já saiu).
- Acessibilidade: a coluna é um `nav` com rótulo "Contas", cada item é um botão com `aria-current` na ativa e nome acessível "Trabalho, joao@empresa.com, 12 não lidas", e a navegação por setas funciona dentro da coluna.
- Conta com `status = needs_reauth`: ponto de alerta no avatar e, ao abri-la, a section message "Reconecte esta conta" com o botão "Reconectar" (refaz o OAuth para o mesmo e-mail sem apagar o banco).

**CA6. Sair de uma conta (dados locais mantidos)**

- "Sair desta conta" abre um modal: "Sair de Trabalho (joao@empresa.com)? O Epistula deixa de acessar esta conta. As mensagens já salvas continuam neste computador. Nada muda no Gmail." com "Sair" e "Cancelar". O foco começa em "Cancelar".
- Ao confirmar, nesta ordem:
  1. encerra a sincronização e a sessão IMAP da conta e descarta o access token da memória;
  2. revoga o refresh token em `https://oauth2.googleapis.com/revoke` (melhor esforço: sem rede, segue em frente e registra só o código do erro);
  3. apaga os segredos da conta no keyring;
  4. marca a conta como `signed_out` no banco. Pastas, mensagens, corpos, marcadores e ajustes **permanecem**, cifrados pelo SQLCipher.
- Se a etapa 3 falhar, a conta fica `signed_out` mesmo assim e a remoção do segredo é tentada de novo na próxima abertura do app. Uma conta `signed_out` nunca usa um segredo que tenha sobrado.
- Conta `signed_out`:
  - continua na coluna, esmaecida, sem contador de não lidas;
  - as mensagens salvas podem ser lidas, e as pastas navegadas, a partir do banco, com a section message "Você saiu desta conta" e o botão "Entrar de novo";
  - não sincroniza e não aceita ações que precisam do servidor (mudar flag, abrir mensagem cujo corpo não foi salvo), que mostram `SignedOut`;
  - "Entrar de novo" refaz o OAuth. Se o Google devolver o mesmo e-mail, a conta volta a `active` e a sincronização continua de onde parou. Se devolver outro e-mail, esse vira uma conta nova e a antiga fica como estava.
- Depois de sair, a conta continua selecionada. Trocar de conta segue funcionando normalmente.
- Apagar os dados de uma conta do computador **não** faz parte desta story (ver D4).
- O comando `disconnect` da US1 é substituído por `sign_out_account { accountId }`.

**CA7. Segredos por conta e migração da US2**

- No keyring, por conta: `account-<id>` (metadados) e `secret-<id>-N` (refresh token em partes de 1000 caracteres, como na US1). Mais a entrada global `db-key`.
- Na primeira execução desta versão, a entrada única da US1/US2 (`account` + `secret-N`) vira uma conta no banco com nome padrão e cor azul, e as entradas antigas são apagadas só depois que as novas forem gravadas e lidas com sucesso.

**CA8. Privacidade e logs**

- Logs identificam a conta só pelo `account_id` (UUID). Nunca registram e-mail da conta, nome dado à conta, assunto, endereços, corpo, nome de anexo ou de marcador, chave do banco ou token. Vale também para mensagens de erro do SQLite repassadas ao log.
- O teste de redaction da US2 é estendido: cria duas contas sintéticas no banco, sincroniza mensagens marcadas (via camada de armazenamento, sem IMAP), troca de conta, sai de uma conta e falha se qualquer marcador aparecer no log.
- O arquivo do banco, aberto sem a chave, não contém nenhum desses marcadores (teste lendo os bytes do arquivo).

**CA9. Erros novos**

Seguem o contrato `{ kind, message }`:

- `AccountAlreadyConnected`: "Esta conta já está no Epistula."
- `AccountNotFound`: id de conta inexistente (erro de programação).
- `SignedOut`: "Você saiu desta conta. Entre de novo para atualizar as mensagens."
- `ProviderUnavailable`: provedor ainda não suportado.
- `Storage`: falha de banco. Mensagem genérica "Não foi possível acessar os dados salvos neste computador", sem texto do SQLite.
- `Offline`: sem rede e o dado não está no banco.

### 6. Comandos Tauri (contrato)

| Comando | Entrada | Saída |
|---|---|---|
| `list_accounts` | — | `Account[]` (`{ id, provider, email, displayName, color, position, status, unread }`) |
| `list_providers` | — | `Provider[]` (`{ id, name, available }`) |
| `connect_account` | `{ provider }` | `Account` (OAuth da US1; cria ou reutiliza a conta) |
| `update_account` | `{ accountId, displayName?, color?, position? }` | `Account` |
| `sign_out_account` | `{ accountId }` | `Account` (com `status = signed_out`) |
| `reconnect_account` | `{ accountId }` | `Account` (para `needs_reauth` e `signed_out`) |
| `list_folders` | `{ accountId }` | `Folder[]` |
| `list_messages` | `{ folderId, cursor? }` | `{ items, nextCursor?, total, unread, syncing }` |
| `sync_folder` | `{ folderId }` | `void` (resultado chega pelo evento `mailbox-updated`) |
| `get_message`, `set_flags`, `open_external_link` | iguais à US2 (ids agora carregam a conta) | iguais à US2 |
| `get_account`, `disconnect` | — | Removidos: substituídos por `list_accounts` e `sign_out_account` |

Evento: `mailbox-updated { accountId, folderId }`, e `account-status { accountId, status }` quando uma conta passa a precisar de reconexão.

### 7. Entregáveis além do código

1. `docs/arquitetura-e-ameacas.md` atualizado:
   - banco e chave do banco na matriz de dados;
   - ameaça 3 (roubo de banco ou backup) passa de "não há banco" para "banco cifrado, chave no keyring";
   - risco aceito pela D4: depois de sair, as mensagens da conta continuam no disco (cifradas); qualquer pessoa com acesso à sessão do usuário no Windows consegue lê-las pelo Epistula;
   - ameaça 7 (dispositivo comprometido) registra que o banco aberto fica legível para malware na sessão do usuário;
   - ameaça 11 cobre WAL, arquivos temporários e `secure_delete`.
2. Diagrama do esquema (tabela e relações) no mesmo documento.
3. `cargo deny` e `cargo audit` com as novas dependências (`rusqlite`, OpenSSL vendorizado), incluindo a licença do SQLCipher (BSD) no `deny.toml`.

### 8. Fora do escopo

- Conectar Outlook, Yahoo, iCloud, Zoho, GMX, AOL ou domínio próprio (o modelo já suporta; cada um terá story).
- Caixa de entrada unificada (todas as contas numa lista só).
- Busca (usará este banco com FTS5).
- Fila de ações offline, envio, rascunhos.
- Download de anexos.
- Exportar ou fazer backup do banco.
- Sincronizar todas as pastas o tempo todo (só a Caixa de entrada é sincronizada periodicamente; as demais, ao abrir).

### 9. Decisões pendentes (regra 4 do prompt)

**D1.** Continua pendente: tipo de produto (US2, seção 9). Esta story segue a opção A.

**D2. Banco cifrado: decidido em 23 out 2026 — SQLCipher.**

Benchmark nesta máquina (50 mil cabeçalhos, mil corpos de 50 KB, mediana em ms, duas rodadas consistentes):

| Operação | SQLite comum | SQLCipher | Cifra por coluna (XChaCha20-Poly1305) |
|---|---:|---:|---:|
| Listar página (50) | 0,07 | 0,13–0,15 | 0,43 |
| Abrir corpo salvo | 0,05 | 0,21 | 0,11 |
| Abrir banco + 1ª página | 1,7 | 2,1 | 2,1 |
| Gravar 500 cabeçalhos | 2,3 | 3,1 | 7,4 |
| Marcar 500 como lidas | 9,2 | 22 | 10,4 |
| Buscar no assunto, sem índice (60 mil) | 21 | 61 | ~200 |
| Ordenar por remetente, sem índice | 34 | ~80 | ~200 |

O SQLCipher fica abaixo de 1 ms nas ações do dia a dia e protege o arquivo inteiro, incluindo índices e metadados. A cifra por coluna vaza metadados, impede índices e busca (FTS5) e exigiria um esquema criptográfico próprio. Ressalva: o SQLCipher foi medido com o OpenSSL 1.1.1 do sistema; o binário final usa OpenSSL embutido e é medido de novo (CA1).

**D3.** E2EE: continua pendente (US2).

**D4. Ao sair, os dados locais: decidido em 23 out 2026 — manter.** Atualização de 25 out 2026: a conta de que o usuário saiu some da coluna de contas; os dados continuam cifrados no banco e voltam ao entrar de novo com o mesmo e-mail. Sair encerra o acesso e apaga os segredos, mas as mensagens ficam no banco cifrado (CA6). Uma opção separada para apagar os dados de uma conta do computador, se for desejada, fica para outra story.

**D5. Quanto guardar: decidido em 23 out 2026 — as 2.000 mensagens mais recentes de cada pasta na primeira sincronização, mais as que o usuário abrir ou paginar depois.** Sem limite de idade nem de tamanho total. Corpos são baixados ao abrir.

### 10. Testes obrigatórios

- **Banco (integração, SQLite real em diretório temporário):**
  - migrações do zero e a partir de cada versão anterior;
  - falha no meio de uma migração não altera `user_version`;
  - chave errada é detectada e o arquivo é posto de lado (CA1);
  - `UNIQUE(provider, email)` impede conta duplicada;
  - sair da conta mantém todas as linhas dela e muda só o `status`; entrar de novo com o mesmo e-mail reaproveita a conta;
  - a primeira sincronização simulada de uma pasta com 5.000 mensagens grava exatamente as 2.000 mais recentes;
  - mensagem em duas pastas é gravada uma vez e sai do banco só quando sai da última pasta;
  - id opaco de uma conta é rejeitado em outra;
  - o arquivo do banco não contém marcadores em texto claro (CA8).
- **Sincronização (lógica pura, sem IMAP):** a partir de um estado salvo e de uma resposta simulada do servidor, calcula corretamente novas, alteradas e removidas, inclusive com `UIDVALIDITY` alterado.
- **Proptest:** sequências aleatórias de inserir, mover, mudar flag e remover mantêm o banco consistente (contadores de não lidas iguais à contagem real).
- **Redaction:** CA8.
- **Frontend (vitest):** ordem e atalhos da coluna, rótulo acessível, restauração do estado por conta, iniciais a partir do nome da conta, rollback de flag com banco.
- **Manual contra o Gmail real**, com 3 contas de teste:
  - conectar as 3, nomear "Pessoal", "Trabalho" e "Família" com cores diferentes;
  - trocar entre elas por clique e por `Ctrl+1/2/3`;
  - desligar a rede e abrir mensagens já lidas;
  - receber um e-mail novo e vê-lo aparecer em até 5 minutos sem clicar em nada;
  - marcar estrela no Gmail web e ver refletido;
  - revogar o acesso de uma conta em myaccount.google.com e ver o alerta de reconexão;
  - sair de uma conta, confirmar que o segredo some do keyring e o acesso aparece revogado no Google, ler mensagens salvas dela e entrar de novo;
  - atualizar a partir da versão da US2 e confirmar que a conta existente foi migrada sem novo login.

### 11. Arquivos esperados

| Camada | Arquivo | Responsabilidade |
|---|---|---|
| Rust | `src-tauri/src/db/mod.rs` | Abertura com SQLCipher, chave, pragmas, migrações |
| Rust | `src-tauri/migrations/0001_inicial.sql` | Esquema da seção 5 (CA2) |
| Rust | `src-tauri/src/db/repo.rs` | Consultas por tabela, sempre parametrizadas |
| Rust | `src-tauri/src/sync.rs` | Sincronização incremental e eventos |
| Rust | `src-tauri/src/providers.rs` | Catálogo de provedores |
| Rust | `src-tauri/src/accounts.rs` | Contas, segredos por conta, remoção, migração da US2 |
| Rust | `src-tauri/src/session.rs`, `imap_client.rs`, `commands.rs` | Sessões por conta e comandos da seção 6 |
| TS | `src/features/accounts/AccountRail.tsx` | Coluna de contas, atalhos, menu |
| TS | `src/features/accounts/AddAccount.tsx`, `AccountSetup.tsx`, `RemoveAccount.tsx` | Provedores, nome e cor, confirmação de saída |
| TS | `src/ui/AccountRail.tsx`, `src/ui/Modal.tsx`, `src/ui/Menu.tsx` | Wrappers do DS |
| DS | `components.css`, `espacamento.html`, `sidebar.html`, `selecao.html` | L15–L17 |

### 12. Definição de pronto

1. D2, D4 e D5 decididas (23 out 2026).
2. L15–L17 publicadas no DS.
3. Todos os CAs verificados no app com 3 contas Gmail de teste.
4. Testes da seção 10 passando, com o resultado real informado.
5. `cargo clippy`, `tsc --noEmit`, `cargo audit` e `cargo deny check` sem erros, ou com exceções justificadas.
6. Nenhum `unwrap()`/`expect()` em caminhos de banco, rede ou dados do servidor.
7. `docs/arquitetura-e-ameacas.md` atualizado.
8. Atualização a partir da US2 testada sem perder a conta conectada.

### 13. Implementação (23 out 2026)

**Decisões tomadas na implementação**

- **Mesma mensagem em várias pastas:** o `async-imap` não expõe o `X-GM-MSGID`, então a chave é Message-ID + tamanho. Sem Message-ID, a mensagem fica presa à pasta e ao UID. Duas cópias iguais **na mesma pasta** continuam separadas, porque no servidor cada UID tem suas próprias flags (o teste de propriedade achou esse caso).
- **Contas:** conectar uma conta que já existe não é erro. `connect_account` devolve `{ account, isNew, wasActive }` e o app avisa "Esta conta já está no Epistula". Os comandos ficaram `sign_out_account` (no lugar de `remove_account`), `move_account` (subir ou descer uma posição) e `set_last_folder`.
- **Sem rede:** usa o erro `ImapConnection` da US2, sem um tipo `Offline` separado. A sincronização emite `sync-status { online }`, e o app mostra o banner "Sem conexão".
- **Conexões IMAP:** uma interativa (abrir mensagem, flags, páginas antigas) e uma de sincronização por conta.
- **Seletor abaixo de 960px:** o `.account-switcher` está no DS, mas a janela do app tem largura mínima de 960px, então ele não aparece no app.
- **Banco recriado:** se a chave sumir ou não abrir o arquivo, o arquivo é posto de lado e as contas voltam pelo índice no keyring (`accounts-index`), sem novo login quando o refresh token ainda existe.
- **Build no Windows:** `scripts/setup-build-tools.ps1` baixa um Strawberry Perl portátil (SHA-256 fixo) para `.tools/`, e `src-tauri/.cargo/config.toml` aponta `OPENSSL_SRC_PERL` para ele.
- **DS:** além de L15–L17, `.app-shell--with-rail .app-shell__center` (telas de adicionar conta e login ao lado da coluna) e `.modal--warning`.

**Verificado**

- **Rust: 64 testes**, estáveis em 5 execuções seguidas. Incluem:
  - banco real com SQLCipher em diretório temporário: migrações do zero, migração quebrada sem mudar a versão, banco de versão mais nova recusado, chave errada detectada e arquivo posto de lado;
  - unicidade de conta, cores e ordem, nome e cor validados, sair mantendo todas as linhas, reconectar reaproveitando a conta;
  - mensagem em duas pastas guardada uma vez, cópias na mesma pasta separadas, id de outra conta recusado, troca de UIDVALIDITY;
  - primeira sincronização simulada de 5.000 mensagens gravando exatamente as 2.000 mais recentes, e o plano incremental (novas, CONDSTORE, sem CONDSTORE, chegada maior que uma janela);
  - o arquivo do banco (e o WAL) sem texto claro;
  - teste de propriedade (48 casos por execução, mais uma rodada com 1.000) mantendo o banco sem órfãs e os contadores de não lidas corretos;
  - redaction estendida a contas, pastas, corpo e saída de conta;
  - os testes da US2.
- **Frontend: 25 testes** (contas: inicial, `99+`, nome acessível, tooltip, `Ctrl+1`–`9`, cor padrão, conta ativa), `tsc` e build do Vite.
- **Qualidade e dependências:** `cargo clippy --all-targets` sem avisos. `cargo deny check` ok. `cargo audit` sem vulnerabilidades, com os mesmos 8 avisos de crates usadas só no Linux.
- **Interface com backend simulado:** coluna de contas, menu da conta, adicionar conta, nome e cor, sair, conta que precisa reconectar, conta de que o usuário saiu e primeiro uso.

**Benchmark da D2 refeito com o OpenSSL embutido (CA1)**

O build embutido no Windows usa `no-asm` (sem as instruções AES do processador). Mesma carga da D2, mediana em ms, duas rodadas consistentes:

| Operação | SQLite comum | SQLCipher (OpenSSL do sistema, D2) | SQLCipher (OpenSSL embutido, app) |
|---|---:|---:|---:|
| Listar página (50) | 0,07 | 0,13–0,15 | 0,25–0,26 |
| Abrir corpo salvo de 50 KB | 0,05 | 0,21 | 0,59 |
| Abrir banco + 1ª página | 1,7 | 2,1 | 2,3 |
| Gravar 500 cabeçalhos | 2,3 | 3,1 | 4,3 |
| Marcar 500 como lidas | 9,2 | 22 | 47 |
| Buscar no assunto, sem índice (60 mil) | 21 | 61 | 152 |

Listar uma página fica em 0,26 ms, bem abaixo do limite de 20 ms; nada muda na decisão. O custo aparece em varreduras completas (busca sem índice), que a story de busca resolve com FTS5. Instalar o NASM ativaria o assembly do OpenSSL, se um dia for preciso.

O build do OpenSSL falha se o diretório `target` ficar num caminho muito longo (limite de 260 caracteres do Windows); no caminho do projeto funciona.

**Pendente**

- Teste manual contra o Gmail real (seção 10), com três contas.

## User Story 5: Layout em blocos, seleção múltipla, foto da conta e configurações de sincronia

**Como** usuário que organiza várias caixas no Epistula,
**quero** selecionar vários e-mails de uma vez e movê-los para qualquer pasta, ver a foto das minhas contas, escolher de quanto em quanto tempo cada conta procura e-mails novos e ter um layout em blocos sobre um fundo que poderei personalizar,
**para que** eu organize a caixa mais rápido e o app fique com a minha cara.

---

### 1. Contexto

Hoje a lista de e-mails usa a variante "só cabeçalho" do DS (sem checkbox), os comandos agem só na mensagem aberta, a coluna de contas começa abaixo do header, todas as áreas são uma superfície contínua e a sincronização roda a cada 5 minutos fixos. Esta story:

1. usa a linha completa do DS na lista (**checkbox**, hover, marcada) e faz os comandos agirem em **várias mensagens**;
2. permite **arrastar** uma ou mais mensagens para qualquer pasta, além do "Mover para" da barra;
3. muda o **layout** para blocos sobre um fundo (canvas), com a coluna de contas subindo até o topo;
4. traz a **foto** da conta Google;
5. cria a tela de **Configurações** (Geral, Aparência, Conta, E-mail) com a **sincronia por conta**;
6. separa os componentes de layout com nomes claros no código (seção 7).

### 2. Regra do design system (obrigatória)

Vale integralmente a seção 2 da US1. As peças que faltam estão na seção 4 e passam primeiro pelo rascunho `design-system/rascunho-us5.html`.

### 3. Layout

```
┌────┬──────────────────────────────────────────────────────────────────────┐
│ ▣  │╭────────────────────────────────────────────────────────────────────╮│
│    ││ Epistula      │ [🔍 Buscar…        ] ⚙ │            (T) Trabalho   ││ bloco do header
│(T) ││───────────────┴─────────────────────────┴───────────────────────────││
│(P) ││ [✎ Novo e-mail] │ 🗑 📦 📁 Mover ▾ ↩↩ ▾ │ ✉ ⚑ ▾                      ││
│(F) │╰────────────────────────────────────────────────────────────────────╯│
│    │                          ← gap: o fundo aparece →                     │
│ ＋ │╭─────────────────────────────────────╮ ╭────────────────────────────╮│
│    ││ pastas      │ ☐ lista de e-mails    │ │ conteúdo                   ││
│    ││ Entrada  12 │ ☑ Marina  Revisão…    │ │ (ocupa toda a largura do   ││
│    ││ Estrela     │ ☑ Paulo   Almoço…     │ │  bloco)                    ││
│    ││ Enviados    │ ☐ Banco   Extrato…    │ │                            ││
│    │╰─────────────────────────────────────╯ ╰────────────────────────────╯│
└────┴──────────────────────────────────────────────────────────────────────┘
 coluna de contas: cor do fundo, do topo ao rodapé
```

- **Fundo (canvas):** um token novo (`--color-canvas`) que fica atrás de tudo. A casca lê `--app-background`, que no futuro recebe a cor ou a imagem escolhida pelo usuário (temas ficam para outra story, ver D6).
- **Coluna de contas:** sobe até o topo da janela e não tem superfície própria: fica sobre o fundo.
- **Bloco do header:** começa à direita da coluna de contas (o header "encolhe") e agrupa marca, busca, botão de configurações, conta ativa e a barra de comandos (D1) numa superfície elevada.
- **Bloco de navegação:** pastas e lista de e-mails juntas, separadas por uma borda.
- **Bloco de conteúdo:** a leitura. O conteúdo acompanha a largura do bloco, sem a medida de 672px que deixava espaço vazio à direita.
- **Gaps:** 8px entre o header e os blocos, entre os dois blocos e em volta deles. O divisor da lista (US4) fica no gap entre os blocos.
- A busca continua alinhada à coluna da lista, e o botão de configurações fica logo à direita dela.

### 4. Lacunas no design system

| # | Lacuna | Proposta |
|---|---|---|
| L25 | Layout em blocos | `.app-shell--blocks`: fundo `--app-background`, gaps de `--space-100`, coluna de contas em toda a altura, `.app-shell__block` (superfície, raio `--radius-large`, sombra `--shadow-raised`) |
| L26 | Fundo da janela | Token `--color-canvas` (claro e escuro) e a propriedade `--app-background`, que aceita cor ou imagem |
| L27 | Leitura na largura do bloco | `.reader--fluid`: corpo, quadro HTML e resposta sem a medida de leitura |
| L28 | Seleção múltipla na lista | A linha completa do DS (`.thread` com `.thread__check`, já existe). Falta só o modo "seleção" da toolbar da lista (já existe: `.toolbar.is-selecting` com `.toolbar__count`) e o hover mais visível dentro do bloco |
| L29 | Arrastar para uma pasta | `.nav-item.is-drop-target` (pasta sob o cursor) e `.drag-ghost` ("3 conversas") |
| L30 | Avatar com foto | `.avatar` com `<img class="avatar__img">`; a inicial continua por baixo enquanto a foto carrega ou se ela falhar |
| L31 | Botão de configurações | Ícone `settings` (engrenagem) num `.btn--subtle.btn--icon` à direita da busca |
| L32 | Tela de configurações | `.modal--settings`: modal grande com `.settings__nav` (seções em coluna à esquerda, `.nav-item`) e `.settings__panel` à direita, com `.settings__group` e `.setting` (rótulo, descrição e controle) |
| L33 | Lista de opções | `.select` (select nativo com a aparência do `.input`) |
| L34 | Sincronia por conta | `.setting-list` com uma linha por conta (checkbox, avatar, nome e e-mail, `.select` do intervalo) e a barra "Aplicar a N contas" |
| — | Ícones novos | `settings`, `user`, `palette` |

### 5. Critérios de aceite

**CA1. Hover e checkbox na lista**

- Cada linha mostra o checkbox, a estrela, o remetente, a data e o assunto (DS `threads.html`, variante padrão sem prévia).
- O hover destaca a linha inteira e troca a data pelas ações rápidas.
- Marcar o checkbox não abre a mensagem. A linha marcada usa o fundo selecionado; a aberta tem também a barra azul (DS).
- `X` marca ou desmarca a linha em foco; `Shift`+clique no checkbox marca o intervalo desde o último marcado; `Ctrl+A` com a lista em foco marca toda a página.
- O checkbox da toolbar da lista marca ou desmarca a página inteira e fica indeterminado com seleção parcial. Com seleção, a toolbar mostra "3 selecionadas" e o botão "Limpar seleção".
- A seleção é limpa ao trocar de pasta, de conta ou de página, e ao buscar.

**CA2. Comandos em várias mensagens**

- Com uma ou mais linhas marcadas, **Excluir**, **Arquivar**, **Mover para**, **Lida/Não lida** e **Sinalizar** agem em todas as marcadas. Sem marcadas, agem na mensagem aberta (como hoje).
- **Responder a todos ▾** fica desabilitado com mais de uma marcada.
- Lida/Não lida com seleção mista: se alguma estiver não lida, a ação é "Marcar como lida".
- O toast com **Desfazer** vale para o lote inteiro: "3 conversas movidas para Projetos". Desfazer devolve todas.
- Se o servidor recusar parte do lote, as que falharam voltam para a lista e o toast de erro diz quantas.

**CA3. Mover para qualquer pasta**

- O menu "Mover para" e o seletor de pastas da US4 funcionam com o lote.
- **Arrastar:** arrastar uma linha (ou qualquer linha marcada, levando todas as marcadas) até uma pasta da coluna de pastas move para ela. A pasta sob o cursor fica destacada e um rótulo segue o cursor ("3 conversas"). Soltar fora de uma pasta cancela.
- Não aceitam soltar: a pasta atual e "Com estrela".

**CA4. Layout em blocos**

- O layout segue a seção 3 em janelas de 960px ou mais, nos temas claro e escuro.
- Abaixo de 960px, segue o comportamento atual (sem a coluna de contas, lista e leitura dividem o espaço), com os mesmos blocos e gaps.
- O conteúdo da leitura (texto, e-mail HTML e card de resposta) acompanha a largura do bloco quando a janela ou o divisor mudam, sem sobrar espaço à direita.
- O fundo é lido de `--app-background`; trocar essa propriedade (cor ou imagem) muda o fundo sem tocar nos componentes.

**CA5. Foto da conta Google (D4)**

- A coluna de contas e o header mostram a foto da conta Google em vez da inicial.
- Sem foto (conta sem foto, escopo não autorizado, sem rede), continua a inicial na cor escolhida.
- A foto é baixada pelo processo Rust e guardada no banco cifrado, e a WebView recebe uma `data:` URL. Nenhuma requisição sai da WebView para o Google.
- A foto é atualizada ao conectar ou reconectar a conta e, no máximo, uma vez por dia quando o token é renovado.

**CA6. Configurações**

- O botão de engrenagem à direita da busca (tooltip "Configurações") abre a tela de configurações. Atalho: `Ctrl+,`.
- À esquerda, as seções em coluna: **Geral**, **Aparência**, **Conta** e **E-mail**. Clicar numa seção mostra as opções à direita. `↑`/`↓` navegam entre as seções.
- Nesta story só **E-mail → Sincronização** tem opções. As outras seções mostram "Em breve" e o que vai entrar nelas.
- `Esc` ou "Fechar" fecham. As mudanças valem na hora (sem botão "Salvar").

**CA7. Sincronização por conta (D3)**

- **Verificar novos e-mails a cada:** Manualmente, 1, 2, 5 (padrão), 10, 15, 30 ou 60 minutos.
- **Para todas as contas:** o intervalo padrão vale para toda conta sem um intervalo próprio.
- **Por conta:** a lista mostra cada conta com o seu intervalo. Uma conta pode "Usar o padrão" ou ter um intervalo próprio.
- **Para uma lista de contas:** marcar várias contas na lista e escolher um intervalo em "Aplicar a N contas" muda todas de uma vez.
- Contas de que o usuário saiu aparecem desabilitadas, com "Você saiu desta conta".
- O backend agenda cada conta pelo seu intervalo. Mudar o intervalo vale a partir da próxima verificação, sem reiniciar o app. "Manualmente" só sincroniza ao abrir a conta ou ao clicar em "Atualizar".

### 6. Backend (contrato)

| Comando | Entrada | Saída |
|---|---|---|
| `get_sync_settings` | — | `{ defaultMinutes: number \| null, accounts: [{ accountId, minutes: number \| null, usesDefault }] }` (`null` = manual) |
| `set_default_sync_interval` | `{ minutes: number \| null }` | `SyncSettings` |
| `set_account_sync_interval` | `{ accountIds: string[], minutes: number \| null, useDefault: boolean }` | `SyncSettings` |
| `move_messages`, `delete_messages`, `archive_messages`, `set_flags`, `set_followup` | já recebem ou passam a receber uma lista de ids | listas de ids que deram certo e que falharam |

- **Banco (migração 0003):**
  - `app_setting (key, value)`: `sync_default_minutes`;
  - `account_setting` ganha `sync_minutes` por conta;
  - `account` ganha `photo` (BLOB, até 256 KB), `photo_type` e `photo_checked_at`.
- **Agendador:** um laço a cada 15 s escolhe as contas ativas cuja próxima verificação venceu. Cada conta sincroniza no máximo uma vez por vez (lane Sync da US3).
- **Foto:** escopo `profile` no OAuth; a claim `picture` do `id_token` dá a URL. O download aceita só `https` em `*.googleusercontent.com`, até 256 KB, `image/jpeg`, `image/png` ou `image/webp`, com timeout de 10 s.
- **Validação:** intervalos fora da lista são recusados (`InvalidSetting`).

### 7. Organização do código (pedido do usuário)

Componentes de layout com nomes do domínio, um por arquivo, em `src/layout/`:

| Componente | O que é |
|---|---|
| `AppLayout` | A casca em blocos (fundo, áreas e gaps) |
| `AccountsBar` | Coluna de contas (topo ao rodapé) |
| `AccountsBarItem` | Uma conta na coluna |
| `Header` | Bloco do header: agrupa os componentes abaixo |
| `HeaderBrand` | Marca |
| `HeaderSearch` | Busca alinhada à lista |
| `SettingsButton` | Engrenagem à direita da busca |
| `HeaderAccount` | Conta ativa |
| `CommandBar` | Barra de comandos (segunda linha do header) |
| `NavigationBlock` | Bloco com as pastas e a lista |
| `FoldersColumn` | Coluna de pastas |
| `FolderItem` | Uma pasta (Caixa de entrada, Com estrela, Enviados, Rascunhos…) |
| `EmailListColumn` | Coluna da lista: toolbar e `EmailList` |
| `EmailList` / `EmailItem` | A lista e cada e-mail |
| `ContentColumn` | Bloco de conteúdo (leitura, resposta) |

A lógica (carregar pastas, seleção, desfazer) continua em `features/`. As peças visuais do DS continuam em `ui/`.

### 8. Fora do escopo

- Escolher tema, cor ou imagem de fundo (só a base técnica, D6).
- As opções de Geral, Aparência e Conta.
- Notificações de e-mail novo.
- Sincronização em tempo real (IMAP IDLE).
- Selecionar mensagens de várias páginas ou de toda a pasta.

### 9. Decisões

- **D1 (pendente, sugestão):** a barra de comandos fica **dentro** do bloco do header, como segunda linha. O pedido diz que o header agrupa "todos os itens que estão dentro dele", e os comandos são do header desde a US4.
- **D2 (pendente, sugestão):** Configurações abrem num **modal grande** (como o Outlook), por cima do app. A alternativa é ocupar o lugar dos blocos.
- **D3 (pendente, sugestão):** intervalos Manualmente, 1, 2, 5, 10, 15, 30 e 60 min. Menos de 1 min não compensa: o Gmail limita conexões, e IMAP IDLE (tempo real) fica para outra story.
- **D4 (pendente, sugestão):** a foto vem do escopo `profile` (não sensível; não exige verificação do Google) e é guardada no banco cifrado. Contas já conectadas precisam **reconectar** uma vez para autorizar o escopo novo.
- **D5 (pendente, sugestão):** arrastar e soltar para pasta, além do "Mover para".
- **D6 (pendente, sugestão):** nesta story, só o token do fundo e a propriedade `--app-background`; a escolha de tema, cor ou imagem fica em Aparência numa próxima story.

### 10. Configuração necessária (Google Cloud)

Para a foto vir, no projeto `epistula-desktop` do Google Cloud:

1. **Google Auth Platform → Acesso a dados (Data access) → Adicionar ou remover escopos.**
2. Marcar `.../auth/userinfo.profile` ("Ver suas informações pessoais, inclusive as que você disponibilizou publicamente"). É um escopo não sensível, sem verificação adicional.
3. Salvar. O `client_id` e o `client_secret` não mudam.
4. No Epistula, **reconectar cada conta** já conectada (menu da conta → Reconectar) para aceitar o escopo novo. Contas novas já pedem o escopo.

Enquanto o app estiver em modo Testing, nada mais muda.

### 11. Testes obrigatórios

- **Rust:**
  - intervalos válidos e inválidos; conta sem intervalo usa o padrão; "Manualmente" nunca vence;
  - agendador escolhe só as contas vencidas e ativas;
  - aplicar um intervalo a várias contas de uma vez; id de outra instalação recusado;
  - claim `picture` lida do `id_token`; URL fora de `googleusercontent.com`, `http`, tipo errado e tamanho acima do limite recusados;
  - foto não vai para os logs (redaction estendida);
  - mover, excluir, arquivar e sinalizar em lote, com falha parcial.
- **Frontend:**
  - seleção: marcar, intervalo com Shift, página inteira, estado indeterminado, limpar ao trocar de pasta;
  - qual conjunto o comando usa (marcadas ou aberta);
  - lida/não lida com seleção mista;
  - desfazer de um lote.
- **Manual contra o Gmail real:**
  - mover 3 mensagens arrastando e pelo menu, e conferir no Gmail web;
  - excluir e arquivar em lote com e sem "Desfazer";
  - reconectar e ver a foto;
  - mudar o intervalo de uma conta para 1 min e ver um e-mail novo chegar sem clicar em "Atualizar".

### 12. Definição de pronto

1. Rascunho de design aprovado e incorporado ao DS (L25–L34).
2. Todos os CAs verificados no app com uma conta Gmail.
3. Testes da seção 11 passando, com resultado real informado.
4. `cargo clippy`, `tsc --noEmit`, `cargo deny` e `cargo audit` sem erros.
5. Logs sem a URL da foto, intervalos por conta associados a e-mail, ou conteúdo de mensagens.

### 13. Implementação (24 out 2026)

**Decisões confirmadas pelo usuário:** D1, D3, D4, D5 e D6 como sugerido. **D2:** as Configurações abrem por cima das duas colunas (bloco de navegação e bloco de conteúdo), sem cobrir a coluna de contas. O header continua visível; a busca e os comandos ficam inertes enquanto elas estão abertas, e a engrenagem fecha.

**Design system** (rascunho aprovado, incorporado e apagado):

- `tokens.css`: `--color-canvas` (claro e escuro).
- `components.css`: `.app-shell--blocks`, `.app-shell__block` (`--header`, `--nav`), `.reader--fluid`, hover da `.thread` com `--color-bg-neutral-subtle-hovered`, `.nav-item.is-drop-target`/`.is-drop-blocked`, `.drag-ghost`, `.avatar__img`, `.select`, `.app-shell__sheet` + `.settings-sheet` (no lugar do `.modal--settings` do rascunho, por causa da D2), `.settings*`, `.setting*` e `.setting-list*`.
- Páginas: `espacamento.html` (casca em blocos), `threads.html` (hover e seleção), `sidebar.html` (arrastar), `identidade.html` (foto), `campos.html` (select), `header.html` (botão de configurações), `modais.html` (configurações).
- Ícones `settings`, `user` e `palette`.

**Código**

- Layout em `src/layout/`: `AppLayout`, `AccountsBar`, `AccountsBarItem`, `Header`, `HeaderBrand`, `HeaderSearch`, `SettingsButton`, `HeaderAccount`, `CommandBar`, `NavigationBlock`, `FoldersColumn`, `FolderItem`, `EmailListColumn`, `EmailList`, `EmailItem` e `ContentColumn`. Saíram `features/accounts/AccountsRail`, `features/inbox/InboxList`, `features/mailbox/FolderNav`, `features/mailbox/MailCommands` e `ui/AccountRail`.
- O `Mailbox` continua orquestrando a caixa (seleção, lote, arraste, desfazer) e monta o header pelo `renderHeader` do `App`, para a barra de comandos ficar dentro do bloco do header.
- Configurações em `features/settings/` (`Settings`, `SyncSettings`); primitivas em `ui/SettingsSheet`, `ui/Select` e `ui/AccountAvatar`.
- Backend: migração `0003_sincronia_e_foto.sql`; `photo.rs`; agendador por conta em `sync.rs` (`due_accounts`, volta a cada 15 s); comandos `get_sync_settings`, `set_default_sync_interval`, `set_account_sync_interval`; `move/delete/archive_messages` devolvem `{ ids, failed }`; `set_followup` recebe uma lista.
- `tauri.conf.json`: `dragDropEnabled: false` na janela. Sem isso, o WebView2 entrega os eventos de arrastar ao Tauri (para soltar arquivos) e o arrastar de mensagens não funciona.

**Decisões tomadas na implementação**

- **Falha parcial:** cada pasta de origem é uma operação no servidor. As que falham voltam para a lista com o toast "N conversas não puderam ser movidas"; se nada saiu, o erro volta como antes.
- **Excluir com mensagens da Lixeira na seleção:** pede confirmação ("serão apagadas do Gmail…; as outras vão para a Lixeira") e não tem "Desfazer".
- **Arquivar** ignora as marcadas que já estão em "Todos os e-mails".
- **Responder** com uma marcada abre essa mensagem e o card; com mais de uma, fica desabilitado.
- **Foto:** sem a claim `picture` (escopo não autorizado), a foto salva continua. Sem rede, tenta de novo na próxima renovação do token. Uma foto recusada (tipo, tamanho ou host) é apagada.
- **"Manualmente"** não entra no agendador nem na abertura do app; a pasta aberta continua sincronizando ao abrir (comportamento da US3).

**Verificado**

- **Rust: 84 testes** (6 novos), estáveis em 3 execuções: intervalos (padrão, por conta, em lote, inválidos, conta inexistente desfazendo o lote), agendador (só contas ativas e vencidas, manual nunca vence), foto (claim, host, `http`, porta, usuário na URL, tipos, `data:` URL, verificação diária, `photo_type` restrito no banco) e a redaction de logs estendida a foto e intervalos.
- **Frontend: 45 testes** (10 novos): seleção (marcar, intervalo com Shift nos dois sentidos, página inteira, estado parcial, limpar ids que saíram), alvo dos comandos, lida/não lida mista, títulos no singular e plural, intervalos (rótulos e ida e volta).
- **Qualidade:** `cargo clippy` sem avisos, `cargo deny` ok, `cargo audit` sem vulnerabilidades (8 avisos permitidos, os mesmos de antes), `tsc` e build ok.
- **Interface com backend simulado** (claro e escuro): casca em blocos, fundo trocado por `--app-background`, 3 conversas marcadas com hover, menu "Mover para" com seleção, arrastar 2 conversas até "Projetos" (destaque e rótulo) e soltar (saem da lista), Configurações com "Aplicar a 2 contas", tela de adicionar conta e resposta na largura do bloco.

**Correção: janela branca ao abrir (24 out 2026)**

- **Causa:** o `index.html` carregava a folha de estilos do Google Fonts no `<head>`. O navegador não desenha nada até essa requisição terminar, então a janela ficava branca enquanto a rede respondia (ou até o tempo esgotar, sem rede). Além disso, a sincronização automática de todas as contas começava junto com a abertura e disputava o banco com as primeiras consultas da tela.
- **Correção:**
  - fontes Inter e JetBrains Mono empacotadas com o app (`@fontsource`), e a CSP não permite mais `fonts.googleapis.com` nem `fonts.gstatic.com`;
  - `tokens.css` e `components.css` entram pelo `<head>`, e o `index.html` já traz a tela de carregamento do DS, que aparece antes do JavaScript;
  - a inicialização do backend (keyring, limpeza) roda em segundo plano, e só `list_accounts` espera por ela (medido: 17 ms);
  - a sincronização automática espera o frontend avisar que a primeira tela apareceu (`app_ready`), com limite de 10 s.

**Pendente**

- Configurar o escopo `userinfo.profile` no Google Cloud e reconectar as contas (seção 10).
- Teste manual contra o Gmail real (seção 11).

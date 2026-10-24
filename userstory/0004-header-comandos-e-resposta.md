## User Story 4: Busca no header, barra de comandos, coluna ajustável e resposta

**Como** usuário que lê e organiza e-mails no Epistula,
**quero** buscar mensagens a partir do header, ter os comandos principais (novo e-mail, excluir, arquivar, mover, responder, lida, sinalizar) sempre à mão e ajustar a largura da lista,
**para que** eu organize a caixa sem sair do teclado e do mouse, num layout parecido com o do Outlook.

---

### 1. Contexto

Hoje o header só mostra a marca e a conta ativa, as ações sobre a mensagem estão espalhadas (estrela e "marcar como não lida" no painel de leitura) e a lista tem largura fixa. Esta story:

1. coloca a **busca no header**, com a mesma largura e o mesmo alinhamento da coluna de e-mails;
2. torna a **coluna de e-mails redimensionável**, e a busca acompanha;
3. cria uma **barra de comandos** abaixo do header (decisão D1 desta story);
4. implementa **mover** (com pastas recentes, criar pasta e escolher entre todas), **excluir**, **arquivar**, **lida/não lida** e **sinalizar com prazo**;
5. abre os cards de **responder, responder a todos e encaminhar** dentro do painel de leitura, com rascunho local. O envio fica para a próxima story (D3).

### 2. Regra do design system (obrigatória)

Vale integralmente a seção 2 da US1. As peças que faltam estão na seção 4 e passam primeiro por um rascunho para aprovação (`design-system/rascunho-us4.html`), como na US3.

### 3. Layout

```
┌───────────────────────┬──────────────────┬─────────────────────────────────┐
│ Epistula              │ [🔍 Buscar…    ] │            (T) Trabalho · e-mail│ header
├───────────────────────┴──────────────────┴─────────────────────────────────┤
│ [✎ Novo e-mail] │ 🗑 Excluir  📦 Arquivar  📁 Mover ▾  ↩↩ Responder a todos ▾ │ ✉ Lida  ⚑ Sinalizar ▾ │ comandos
├────┬──────────────────┬──────────────────┬┬────────────────────────────────┤
│rail│ pastas           │ lista            ││ leitura                         │
│    │                  │                  ││  (card de resposta abaixo da    │
│    │                  │                  ││   mensagem; de encaminhamento,  │
│    │                  │                  ││   acima)                        │
└────┴──────────────────┴──────────────────┴┴────────────────────────────────┘
                                            ↑ divisor arrastável
```

- O header usa as mesmas colunas da casca. A busca ocupa exatamente a coluna da lista; a conta ativa fica sobre a coluna de leitura.
- O divisor entre lista e leitura é arrastável (mouse) e ajustável pelo teclado (setas, `Home`/`End`). A largura vai de 20rem a 35rem (os limites atuais do DS) e é lembrada por conta neste computador.
- A barra de comandos ocupa toda a largura abaixo do header.

### 4. Lacunas no design system

| # | Lacuna | Proposta |
|---|---|---|
| L18 | Header alinhado às colunas da casca | `.appbar--shell`: header em `subgrid` com as colunas do `.app-shell`, com `.appbar__search` na coluna da lista |
| L19 | Barra de comandos | `.commandbar` (área `__commands` do `.app-shell`), com grupos (`.commandbar__group`), separador vertical (`.toolbar__sep`, já existe) e botões com rótulo e ícone. Os botões com menu usam `.btn-split` (já existe) ou um chevron no próprio botão |
| L20 | Divisor arrastável | `.app-shell__resizer`: `role="separator"` focável, com `aria-valuenow`, cursor `col-resize` e realce no hover e no foco |
| L21 | Menu "Mover para" | `.menu` com busca (`.menu__search`), seções com título (`.menu__heading`, já existe), criação inline (`.menu__form`: campo + "Salvar") e o item "Selecionar outra pasta…" |
| L22 | Seletor de pastas | Modal com busca e lista de pastas com rolagem (`.folder-picker`, seleção única) e os botões "Nova pasta", "Mover" e "Cancelar" |
| L23 | Resposta dentro da leitura | `.compose--inline`: a `.compose` sem sombra, na largura do painel, com Para/Cc/Cco em `.input--line.chip-input`, Assunto e corpo. Para encaminhar, `.message__quote` com De, Enviado, Para e Assunto da mensagem original |
| L24 | Sinalização com prazo | `.menu-item__meta` (data à direita do item, ex. "sex, 24 out") e o ícone `flag`. Na lista e na leitura, a mensagem sinalizada mostra o prazo em `.lozenge` ("Hoje", "Amanhã", "Atrasada") |
| — | Ícones novos | `flag`, `reply-all`, `folder-plus`, `folder-move` |

### 5. Critérios de aceite

**CA1. Busca no header**

- O campo `.input--search` fica na coluna da lista, com a mesma largura e alinhamento, inclusive enquanto o divisor é arrastado.
- `Ctrl+E` ou `/` levam o foco para a busca; `Esc` limpa e volta para a lista.
- A busca roda no **banco local** da conta ativa (assunto, remetente e destinatários), em todas as pastas, a partir de 2 caracteres, com 250 ms de espera depois da última tecla. A lista mostra "Resultados para …" com a pasta de cada mensagem; limpar a busca volta à pasta onde o usuário estava.
- Mensagens que ainda não foram baixadas do servidor não aparecem (busca no servidor fica para outra story). A tela de resultado diz isso: "Buscando nas mensagens salvas neste computador".

**CA2. Coluna da lista ajustável**

- Arrastar o divisor muda a largura da lista e, por consequência, a da leitura e a da busca.
- Pelo teclado: com o divisor focado, setas mudam 16px por vez (`Shift` + seta, 64px), e `Home`/`End` vão aos limites. O leitor de tela anuncia a largura em porcentagem.
- Duplo clique no divisor volta à largura padrão (25rem).
- A largura fica salva por conta (`account_setting`, chave `list_width`) e é restaurada ao abrir o app e ao trocar de conta.

**CA3. Barra de comandos**

- Ordem: **Novo e-mail** (botão principal, azul) | separador | **Excluir**, **Arquivar**, **Mover para ▾**, **Responder a todos ▾** | separador | **Lida/Não lida**, **Sinalizar ▾**.
- Os comandos de mensagem ficam desabilitados sem mensagem selecionada, em conta de que o usuário saiu e em conta que precisa reconectar ("Novo e-mail" continua habilitado).
- Atalhos: `N` novo e-mail, `Delete` ou `#` excluir, `E` arquivar, `V` mover, `R` responder, `Shift+R` responder a todos, `Shift+F` encaminhar, `Shift+U` lida/não lida (já existe), `Insert` sinalizar. Todos aparecem no tooltip.
- Em janelas estreitas os rótulos somem e ficam só os ícones, com tooltip.

**CA4. Excluir e arquivar**

- **Excluir** move para a Lixeira; na própria Lixeira, pede confirmação e apaga de vez (modal de perigo, como "Esvaziar a lixeira").
- **Arquivar**, no Gmail, tira da Caixa de entrada e deixa em "Todos os e-mails".
- A mensagem some da lista na hora, a seleção vai para a próxima, e aparece um toast com **Desfazer** por 6 segundos. A operação no servidor só roda quando o toast fecha. Desfazer apenas cancela, sem nenhuma chamada ao servidor.
- Se o servidor recusar, a mensagem volta para a lista e aparece um toast de erro com "Tentar de novo".

**CA5. Mover para**

- O botão abre um menu com quatro seções:
  1. **busca** de pastas por nome (filtra as seções abaixo enquanto digita);
  2. **Mais recentes**: as 5 pastas para onde o usuário mais recentemente moveu mensagens nesta conta;
  3. **Criar nova pasta**: campo de nome + "Salvar", que cria a pasta no servidor e move a mensagem para ela;
  4. **Selecionar outra pasta…**, que abre o seletor (L22) com busca, a lista completa de pastas e os botões **Nova pasta**, **Mover** e **Cancelar**.
- Não aparecem como destino: a pasta atual e "Com estrela" (que no Gmail é a estrela, não uma pasta).
- Mover segue o mesmo fluxo de "Desfazer" do CA4.
- Nome de pasta nova: 1 a 100 caracteres, sem `/` no início, sem caracteres de controle; o backend codifica em UTF-7 modificado. Nome repetido mostra "Já existe uma pasta com esse nome".

**CA6. Lida/Não lida**

- Alterna o estado da mensagem selecionada, com a mesma atualização otimista da US2. O rótulo do botão mostra a ação que vai acontecer ("Marcar como não lida" / "Marcar como lida").

**CA7. Sinalizar com prazo (decisão D2)**

- **Sinalizar ▾** abre: Hoje, Amanhã, Esta semana (sexta), Próxima semana (segunda), Sem data, separador, Marcar como concluída, Limpar sinalização. Cada item mostra a data calculada à direita.
- Clicar no botão (fora do chevron) alterna "Sem data" / "Limpar".
- No Gmail, qualquer sinalização liga a estrela (`\Flagged`); "Concluída" e "Limpar" desligam. O prazo e o "concluída" ficam no banco local (`message_followup`), porque o Gmail não guarda prazo.
- Na lista e na leitura, o prazo aparece como lozenge ("Hoje", "Amanhã", "sex, 24 out" ou "Atrasada").
- Se a estrela for tirada em outro dispositivo, a sincronização apaga o prazo local.

**CA8. Responder, responder a todos e encaminhar (só interface, D3)**

- **Responder a todos ▾** é um botão dividido: a parte principal responde a todos, e o menu tem Responder, Responder a todos e Encaminhar.
- **Responder / Responder a todos:** um card abre **abaixo** da mensagem, no painel de leitura, com:
  - **Para**, já preenchido com o remetente. Em responder a todos, também com os demais destinatários, menos a própria conta;
  - **Cc**, também preenchido em responder a todos, e **Cco**, ambos por "Cc/Cco" para mostrar;
  - **Assunto**, preenchido com "Re: <assunto>" (sem repetir "Re:") e editável;
  - **corpo** em texto;
  - **Descartar**, com confirmação se houver texto.
- **Encaminhar:** o card abre **acima** da mensagem, com Para (vazio), Cc/Cco, Assunto "Fw: <assunto>" editável, corpo, e abaixo do corpo o bloco da mensagem original: De, Enviado, Para, Assunto e o texto. Também tem **Descartar**.
- Endereços viram chips ao digitar `,`, `;`, `Enter` ou ao sair do campo. Endereço inválido fica no chip de erro (DS `chip.is-invalid`).
- O rascunho é salvo no banco local (`draft`) a cada 2 segundos de pausa e reaparece ao reabrir a mensagem. Só um rascunho por mensagem e por tipo.
- **Enviar** aparece desabilitado, com o tooltip "O envio chega na próxima versão".
- **Novo e-mail** abre o mesmo card, vazio, no painel de leitura (substitui a mensagem aberta), também salvo como rascunho local.

### 6. Backend (contrato)

| Comando | Entrada | Saída |
|---|---|---|
| `search_messages` | `{ accountId, query }` | `{ items: MessageHeader[] (com a pasta), truncated }` (até 200) |
| `move_messages` | `{ messageIds, targetFolderId }` | `{ moved: string[] }` |
| `delete_messages` | `{ messageIds }` | `{ deleted: string[] }`: move para a Lixeira ou, já na Lixeira, apaga de vez |
| `archive_messages` | `{ messageIds }` | `{ archived: string[] }` |
| `create_folder` | `{ accountId, name }` | `Folder` |
| `recent_folders` | `{ accountId }` | `Folder[]` (até 5) |
| `set_followup` | `{ messageId, followup: "today" \| "tomorrow" \| "this_week" \| "next_week" \| "no_date" \| "complete" \| "clear" }` | `{ flags, followup }` |
| `save_draft` / `get_draft` / `delete_draft` | `{ accountId, kind, inReplyTo?, to, cc, bcc, subject, body }` | `Draft` |
| `set_account_setting` | `{ accountId, key: "list_width", value }` | `void` |

- **IMAP:** `UID MOVE` (RFC 6851, o Gmail suporta), com `COPY` + `STORE \Deleted` + `UID EXPUNGE` quando não houver suporte. A exclusão definitiva usa `STORE \Deleted` + `UID EXPUNGE`. `CREATE` para pasta nova, com o nome em UTF-7 modificado.
- **Banco (migração 0002):**
  - `folder_usage (account_id, folder_id, used_at)`: as "Mais recentes" do CA5;
  - `message_followup (message_id, due_date, completed_at)`: o prazo do CA7;
  - `draft (id, account_id, kind, in_reply_to, to_json, cc_json, bcc_json, subject, body, updated_at)`;
  - `account_setting` para `list_width`.
- **Atualização local:** depois de mover, excluir ou arquivar, o banco é atualizado na hora (a ligação sai da pasta de origem) e a pasta de destino é sincronizada em segundo plano.

### 7. Fora do escopo

- Envio (SMTP), anexos na resposta, formatação rica (D3).
- Busca no servidor e operadores de busca (`de:`, `tem:anexo`).
- Seleção múltipla na lista (os comandos agem na mensagem aberta).
- Lembretes e notificações de prazo.

### 8. Decisões

- **D1 (decidido em 23 out 2026):** comandos numa barra abaixo do header, como no Outlook.
- **D2 (decidido):** a sinalização tem prazos de acompanhamento guardados localmente; no Gmail, vira a estrela.
- **D3 (decidido):** nesta story, só a interface de resposta, com rascunho local; o envio por SMTP fica para a próxima.
- **D4 (pendente, sugestão):** a ordem dos campos no card de resposta. O pedido descreve corpo, depois assunto, depois destinatários. O rascunho segue a ordem usual (Para, Cc/Cco, Assunto, corpo), a mesma do encaminhar e da `.compose` do DS. Se preferir a ordem do pedido, trocamos no rascunho.

### 9. Testes obrigatórios

- **Rust:**
  - assunto "Re:"/"Fw:" sem duplicar (`Re: Re:`, `RE:`, `Fwd:`);
  - destinatários de responder a todos sem a própria conta e sem duplicatas;
  - datas dos prazos (sexta desta semana, segunda da próxima, virada de mês e de ano);
  - nome de pasta validado e codificado em UTF-7 modificado (ida e volta com o decodificador da US2);
  - busca local parametrizada com `%` e `_` escapados;
  - as 5 pastas mais recentes por conta;
  - mover, excluir e arquivar atualizando o banco;
  - id de outra conta recusado;
  - o prazo apagado quando a estrela some na sincronização.
- **Frontend:**
  - largura do divisor (limites, teclado, porcentagem anunciada);
  - parse de endereços em chips;
  - filtro do menu "Mover para";
  - fila de "Desfazer" (cancelar não chama o servidor; fechar o toast chama).
- **Manual contra o Gmail real:**
  - mover para um marcador e conferir no Gmail web;
  - criar pasta;
  - excluir e arquivar com e sem "Desfazer";
  - sinalizar e ver a estrela no Gmail;
  - arrastar o divisor e reabrir o app;
  - buscar uma mensagem salva.

### 10. Definição de pronto

1. Rascunho de design aprovado e incorporado ao DS (L18–L24).
2. Todos os CAs verificados no app com uma conta Gmail.
3. Testes da seção 9 passando, com resultado real informado.
4. `cargo clippy`, `tsc --noEmit`, `cargo deny` e `cargo audit` sem erros.
5. Logs sem nomes de pasta, assuntos, endereços ou termos de busca (o teste de redaction ganha os novos comandos).

### 11. Implementação (23 out 2026)

**Design system** (aprovado no rascunho e incorporado; rascunho apagado):

- `espacamento.html`: `.app-shell--commands`, `.appbar--shell` (subgrid) e `.app-shell__resizer`.
- `header.html`: `.commandbar`.
- `menus.html`: `.menu--wide`, `__search`, `__form`, `__empty`, `.menu-item__meta` e `.lozenge--overdue`.
- `modais.html`: `.folder-picker` e `.modal__foot--split`.
- `composicao.html`: `.compose--inline`, `.compose__toggle` e `.message__quote`.
- Ícones `flag`, `reply-all`, `folder-plus` e `folder-move`.
- D4 ficou com a ordem do rascunho: Para, Cc/Cco, Assunto, corpo.

**Decisões tomadas na implementação**

- **Prazos da sinalização:** calculados no frontend, na data local do usuário. O backend recebe a data pronta (`set_followup { action, due }`) e a valida. Por isso os testes de data estão no vitest, não no Rust.
- **Mensagem movida:** fica marcada como "em trânsito" (`in_transit_since`) por até 10 minutos. Assim a limpeza de órfãs não apaga corpo salvo, prazo e rascunho antes de a pasta de destino ser sincronizada. A sincronização do destino roda logo depois de mover.
- **Busca:** cada mensagem aparece uma vez, na primeira pasta da ordem da sidebar. O resultado traz o id da pasta, para os comandos agirem na pasta certa.
- **Comandos:** os botões de estrela e "não lida" do painel de leitura saíram, porque a barra de comandos agora faz isso.
- **Excluir na Lixeira:** confirma num modal de perigo e não tem "Desfazer".
- **Encaminhar:** o bloco da mensagem original usa o texto puro da mensagem (`body_text`).

**Verificado**

- **Rust: 78 testes**, estáveis em 3 execuções. Os novos cobrem:
  - busca com `%` e `_` literais, sem repetir mensagem e só da conta pedida;
  - mensagem movida mantendo corpo e prazo até chegar ao destino;
  - as 5 pastas recentes por conta;
  - validação de nome de pasta e de data;
  - prazo apagado quando a estrela some na sincronização;
  - rascunho único por mensagem e tipo;
  - largura da lista com limites;
  - UTF-7 modificado na ida e na volta;
  - prefixos Re:/Fw: e destinatários de responder a todos (sem a própria conta, com Reply-To);
  - a redaction de logs estendida a busca, rascunho, sinalização e mover.
- **Frontend: 35 testes:**
  - prazos (sexta, fim de semana, virada de mês e de ano);
  - largura e teclado do divisor;
  - destinatários em chips;
  - fila de "Desfazer" (o servidor só é chamado no fim do prazo; desfazer não chama; uma falha restaura a mensagem).
- **Qualidade e dependências:** `cargo clippy` sem avisos, `cargo deny` ok, `cargo audit` sem vulnerabilidades, `tsc` e build ok.
- **Interface com backend simulado:**
  - header alinhado à lista;
  - barra de comandos;
  - resposta a todos abaixo da mensagem, com chip inválido;
  - encaminhamento acima, com a mensagem original;
  - menu Mover com recentes;
  - seletor de pastas;
  - menu de prazos;
  - busca com a pasta em cada resultado.

**Pendente**

- Teste manual contra o Gmail real (seção 9).

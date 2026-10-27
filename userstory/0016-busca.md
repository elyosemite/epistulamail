## User Story 16: Busca com pessoas, período e filtros, em todas as contas

**Como** usuário do Epistula,
**quero** buscar e-mails por quem enviou ou recebeu, por período e por filtros, numa conta ou em todas,
**para que** eu ache qualquer mensagem guardada no computador sem internet, aja sobre os resultados e guarde as buscas que repito.

---

### 1. Contexto

Pedido em 27 out 2026, antes da IA (adiada). A busca da US4 era só texto: assunto, remetente e começo do texto, numa conta, a partir de 2 letras. As mensagens já ficam no banco cifrado (`epistula.db`), com `to_json` e `cc_json`, as pastas e as marcações; nada disso precisa de rede.

### 2. Critérios de aceite

**CA1. Chips no campo**

- O campo do header aceita chips: pessoa (De, Para, Cc ou em qualquer papel), período, com anexo, não lidas, com estrela e pasta. O texto livre fica no fim.
- Operadores digitados viram o mesmo chip: `de:`, `para:`, `cc:`, `com:`, `depois:`, `antes:`, `em:AAAA`, `tem:anexo`, `é:não-lida`, `é:estrela`, `na:`. Datas em `AAAA-MM-DD` ou `DD/MM/AAAA`.
- Backspace no campo vazio marca o último chip; a segunda vez tira. Esc fecha as sugestões e, depois, limpa a busca.

**CA2. Sugestões**

- Ao digitar um operador de pessoa: os contatos que já escreveram ou receberam, pelo começo do nome ou do endereço, com quantas mensagens; e "Buscar … no remetente" com o que foi digitado.
- Com o campo vazio: as últimas buscas (até 8, só neste computador, com "Limpar") e os filtros com o operador de cada um.

**CA3. Período**

- Atalhos (Hoje, Últimos 7 dias, Últimos 30 dias, Este ano, Qualquer data) e um intervalo com duas datas. Clicar no chip de período abre o mesmo menu.

**CA4. Resultados e onde buscar**

- Barra acima da lista: quantos, "Esta conta | Todas as contas", a ordem (mais recentes ou mais antigas) e "Salvar busca".
- Em todas as contas, cada resultado mostra o avatar da conta (os arquivos de e-mail da US15 entram, com o avatar do arquivo). Abrir um resultado mostra a mensagem com a conta dela; a conta ativa não muda.
- Sem resultados: "Nenhum resultado nesta conta" com "Buscar em todas as contas". Até 200 resultados; passando disso, o aviso pede outro filtro.

**CA5. Ações nos resultados**

- Marcar resultados permite as ações de sempre (arquivar, mover, excluir, lida, sinalizar) quando todos são da mesma conta. Na Lixeira de qualquer conta, excluir pede confirmação.

**CA6. Aplicar regra**

- "Aplicar regra…" escolhe uma regra da conta dos resultados marcados, mostra a prévia (quantos ela pegaria) e aplica só aos que cumprem as condições. O que ela faz vai para o histórico das regras (US12), com "Desfazer".
- "Criar uma regra com esta busca…" abre o editor com De, Para, Cc, Assunto, anexo, lida e estrela vindos dos chips.

**CA7. Buscas salvas**

- "Salvar busca" pede um nome; a busca aparece na coluna de pastas, num grupo próprio, com as não lidas. Clique roda a busca; o menu de contexto exclui.

### 3. Interface

Aprovada no rascunho `design-system/rascunho-us16.html` (mantido, por pedido) e incorporada em `design-system/busca.html`. Peças novas: **L73** chip de filtro (`.chip--filter`) e rótulo do papel (`.chip__op`); **L74** barra dos resultados (`.search-bar`); **L75** conta de cada resultado (`.thread__account`). No header, os chips não quebram linha (os nomes encurtam).

### 4. Decisões

- **D1. Busca no texto inteiro (FTS).** Em aberto. Hoje o texto livre procura no assunto, nas pessoas e no começo do texto (`snippet`). Um índice de texto completo (FTS5 dentro do banco cifrado) aumentaria o banco e o tempo de sincronização; decidir antes de implementar.
- **D2. Mensagens cifradas.** O conteúdo de mensagens OpenPGP e S/MIME não entra em nenhum índice: só o que já é visível sem decifrar (cabeçalhos).
- **D3. Filtro "Cifradas".** Fica fora até a coluna `message.crypto` ser preenchida na sincronização (hoje está vazia).
- **D4. Pessoas do mesmo papel somam.** Dois chips "De" acham uma ou outra; papéis diferentes precisam valer todos.
- **D5. Regra criada da busca.** Como a regra junta as condições com "e", só a primeira pessoa de cada papel entra; "com" e o período não viram condição.
- **D6. Ações em lote numa conta só.** O backend recusa misturar contas; com resultados de várias, as ações ficam desligadas.

### 5. Implementação (27 out 2026)

- Banco: migração `0012_busca.sql` (`saved_search`, com `account_id` nulo para todas as contas).
- Rust: `search.rs` (`SearchQuery` com validação, `search` com até 200 + 1, contatos por `json_each`, recentes em `app_setting`, buscas salvas com as não lidas); `rule_runner::preview_on`/`apply_on` sobre mensagens escolhidas (origem "manual"); comandos `search_messages`, `search_contacts`, `recent_searches`, `add_recent_search`, `clear_recent_searches`, `list_saved_searches`, `save_search`, `delete_saved_search`, `preview_rule_on`, `apply_rule_on`. Logs só com contagens.
- Frontend: `lib/search.ts` (chips, operadores, datas, consulta, regra a partir da busca; testes em `us16.test.ts`), `layout/HeaderSearch.tsx`, `layout/SearchResultsBar.tsx`, `features/search/ApplyRuleDialog.tsx`, buscas salvas em `FoldersColumn`, contas dos resultados no `Mailbox`.

### 6. Fora do escopo

- Busca no servidor (IMAP SEARCH) para mensagens ainda não baixadas.
- Busca dentro de anexos.
- IA (resumos, busca por significado): adiada, com as opções de modelo local, conta do provedor ou MCP.

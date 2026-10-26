## User Story 12: Regras para organizar os e-mails sozinho

**Como** usuário do Epistula,
**quero** criar regras que movam, arquivem, excluam, marquem ou sinalizem e-mails assim que eles chegam,
**para que** a Caixa de entrada fique só com o que importa, sem eu arrumar tudo à mão todo dia.

---

### 1. Contexto

Pedido em 26 out 2026. Hoje a sincronização traz os cabeçalhos novos de cada pasta (`sync.rs`) e, na Caixa de entrada, entrega os ids novos para as notificações (`notify::after_inbox_sync`). Mover, arquivar e excluir já existem com "Desfazer" e retomada depois de fechar o app (`moves.rs`, US7). O IMAP traz `FROM TO CC SUBJECT DATE MESSAGE-ID IN-REPLY-TO REFERENCES CONTENT-TYPE`, o tamanho e as flags; a prévia (`preview.rs`) sabe se há arquivos anexados.

As regras rodam **no Epistula**, durante a sincronização: valem para qualquer provedor (IMAP), mas só agem com o app aberto, inclusive na bandeja. O que chegou com o app fechado passa pelas regras na sincronização seguinte, antes de virar notificação.

### 2. O que os outros clientes fazem (referência)

| Recurso | Outlook | Thunderbird | Gmail | Na US12 |
|---|---|---|---|---|
| Condições por remetente, destinatário, assunto, corpo | Sim | Sim | Sim | Sim (corpo: só o trecho, D3) |
| "Todas" ou "qualquer uma" das condições | Só todas | Sim | Só todas (com OR no texto) | Sim |
| Exceções ("exceto se…") | Sim | Não | "Não tem as palavras" | Sim |
| Domínio, lista de e-mails (`List-Id`), tem anexo, tamanho | Parcial | Sim | Sim | Sim |
| Enviado só para mim, eu em Cc | Sim | Não | Não | Sim |
| Cabeçalho qualquer | Sim | Sim | Não | Avançado (D4) |
| Mover, arquivar, excluir, marcar lida, sinalizar, marcador | Sim | Sim | Sim | Sim |
| Copiar para pasta | Sim | Sim | Marcador | Sim (marcador) |
| Encaminhar, responder automaticamente | Sim | Sim | Encaminhar | Fora (D5) |
| Parar de processar as outras regras | Sim | Sim | Não | Sim |
| Ordem das regras | Sim | Sim | Não | Sim |
| Rodar agora nos e-mails que já estão na pasta | Sim | Sim | "Aplicar às conversas que correspondem" | Sim, com prévia |
| Criar a partir de uma mensagem | Sim | Sim | "Filtrar mensagens assim" | Sim, com sugestões |
| Limpar mantendo só o último de um remetente ("Sweep") | Sim | Não | Não | Sim (CA8) |
| Excluir mais antigos que N dias | Sweep | "Idade em dias" | Não | Sim (CA8) |
| Histórico do que cada regra fez | Não | Log de filtros | Não | Sim, com desfazer |
| Silenciar ou destacar a notificação | Não | Não | Importante | Sim (liga com a US8) |

O que o Epistula faz diferente, pela usabilidade:

- **Prévia antes de salvar:** o editor mostra, enquanto você escreve, quantos e-mails recentes a regra pegaria e quais.
- **Frase legível:** cada regra aparece na lista como uma frase ("De contém @loja.com → Mover para Compras"), não como uma tabela de campos.
- **Desfazer:** tudo que uma regra faz entra no histórico com "Desfazer", e excluir sempre vai para a Lixeira.
- **Sugestões:** criar uma regra a partir de uma mensagem já propõe "deste remetente", "deste domínio", "desta lista" e "com este assunto".

### 3. Regras (obrigatórias)

- Toda peça nova de interface passa primeiro pelo rascunho `design-system/rascunho-us12.html`. Aprovado em 26 out 2026 e incorporado como `design-system/regras.html` (L58 a L63).
- Nenhum texto da interface cita o provedor.
- Regras nunca apagam de vez: "Excluir" manda para a Lixeira, com o mesmo "Desfazer" do mover (US7).
- Regras nunca enviam e-mail (sem encaminhar e sem resposta automática nesta story, D5).
- Logs: só quantas mensagens cada regra afetou, o tipo da ação e tempos; nunca remetente, assunto, texto da condição ou nome de pasta.
- As regras ficam no banco cifrado (US10); com o app bloqueado, não rodam.

### 4. Critérios de aceite

**CA1. Lista de regras (Configurações → Regras)**

- Seção nova nas Configurações, com as regras da conta escolhida no topo (D1) e "Nova regra".
- Cada regra numa linha: liga e desliga, nome, a frase do que faz e quantos e-mails ela afetou nos últimos 30 dias.
- Arrastar ou usar "Subir" e "Descer" muda a ordem; as regras rodam de cima para baixo.
- Menu de cada regra: Editar, Rodar agora, Duplicar, Excluir regra.
- Sem regras: um estado vazio com modelos prontos (CA7).

**CA2. Editor de regra**

- Nome (sugerido a partir das condições, editável).
- "Quando chegar um e-mail em" a conta (e, opcionalmente, outra pasta além da Caixa de entrada, D2).
- Condições, uma por linha: campo, operador e valor. "Todas" ou "Qualquer uma" das condições. Adicionar e remover linhas.
  - Campos: De (endereço ou nome), Domínio do remetente, Para, Cc, Para ou Cc, Enviado só para mim, Assunto, Trecho do texto (D3), Lista de e-mails, Tem arquivos anexados, Tamanho maior que, Já lida, Com estrela.
  - Operadores de texto: contém, não contém, é, começa com, termina com. Sem diferença de maiúsculas e acentos.
  - Vários valores numa linha, separados por vírgula, valem como "qualquer um".
- "Exceto se": as mesmas condições, opcionais.
- Ações, uma ou mais: Mover para pasta, Arquivar, Excluir (Lixeira), Aplicar marcador (copiar para pasta), Marcar como lida, Marcar com estrela, Sinalizar com prazo (hoje, amanhã, em uma semana), Não notificar, Notificar sempre (mesmo no Não perturbe, D6).
- "Parar de aplicar as outras regras" depois desta.
- "Aplicar também aos e-mails que já estão na pasta", com o número de e-mails que seriam afetados.
- Mover e Excluir não convivem com outras ações de pasta na mesma regra (a interface impede e explica).
- **Prévia ao vivo:** "Pegaria 12 dos últimos 500 e-mails", com os 5 primeiros (remetente, assunto e data) e "Ver todos".
- Salvar fica desabilitado sem condição ou sem ação; uma regra sem condição só é aceita com a confirmação "Valer para todos os e-mails".

**CA3. Aplicar ao sincronizar**

- E-mails novos da Caixa de entrada (e das pastas escolhidas, D2) passam pelas regras ativas, em ordem, logo depois de chegarem e **antes** das notificações. O que foi movido, arquivado ou excluído por uma regra não vira notificação de "e-mail novo", salvo "Notificar sempre".
- Cada mensagem passa uma vez só por cada regra (sem reprocessar o que a regra já moveu), e uma mensagem movida para outra pasta não roda as regras dessa pasta de novo.
- As ações usam o caminho do mover (US7) e as flags (US4): a lista atualiza na hora e o servidor recebe em seguida.
- Na primeira sincronização de uma conta, as regras só valem para o que chega a partir dali, salvo "Rodar agora".

**CA4. Rodar agora**

- Em qualquer regra: escolhe a pasta (padrão: a da regra), mostra quantos e-mails serão afetados e pede confirmação.
- Roda em segundo plano, em pacotes, com progresso no toast; pode ser desfeito pelo histórico.

**CA5. Criar a partir de uma mensagem**

- Na barra de comandos e no menu da mensagem: "Criar regra…".
- Abre um editor compacto com sugestões já marcadas: deste remetente, deste domínio, desta lista de e-mails, com este assunto.
- Atalhos de uma ação: "Sempre mover de {remetente} para…" e "Excluir tudo de {remetente}" (este pergunta se vale também para os que já estão na pasta).

**CA6. Histórico e desfazer**

- Toast depois de uma sincronização em que regras agiram: "Regras organizaram 7 e-mails", com "Ver" e "Desfazer".
- Configurações → Regras → Histórico: o que cada regra fez nos últimos 30 dias (data, regra, ação, quantidade), com "Desfazer" em cada lote enquanto as mensagens existirem.
- O sino (US8) registra os lotes grandes (mais de 20 de uma vez) e os erros de regra (pasta que não existe mais).

**CA7. Modelos**

- No estado vazio e em "Nova regra": Newsletters (tem `List-Unsubscribe` → Mover para "Leitura"), Notificações de serviços (remetente `no-reply`/`notifications` → Arquivar e marcar lida), Recibos e faturas (assunto com recibo, fatura, boleto, nota fiscal → Estrela e Sinalizar em uma semana), Pessoas importantes (De é → Notificar sempre e Estrela).
- Um modelo abre o editor já preenchido; nada é salvo sem o usuário confirmar.

**CA8. Limpeza (como o "Sweep")**

- Ações de manutenção, que rodam uma vez por dia e em "Rodar agora": "Manter só o mais recente deste remetente" e "Excluir os mais antigos que N dias" (só da pasta da regra).
- A primeira execução sempre mostra a prévia e pede confirmação.

**CA9. Erros e conflitos**

- Pasta de destino que sumiu: a regra é desligada, aparece com aviso na lista e no sino, e o e-mail fica onde está.
- Sem conexão: as ações ficam na fila do mover (US7) e vão ao servidor quando a conexão voltar.
- Duas regras que mandam a mesma mensagem para pastas diferentes: vale a primeira; o editor avisa quando a nova regra é encoberta por uma de cima.

### 5. Backend

- **Migração 0007:** `rule` (id, conta, nome, posição, ligada, pastas de origem, correspondência todas/qualquer, condições e exceções em JSON com versão, ações em JSON, parar depois, criada e alterada em), `rule_run` (lote: regra, quando, ação, quantidade, se foi desfeito) e `rule_hit` (lote ↔ mensagens, para o desfazer). Tudo com `ON DELETE CASCADE` a partir da conta.
- **`rules.rs`:** tipos `Condition`, `Action` e `Rule`; `matches(rule, &MessageFacts) -> bool` puro e testado (normalização sem acentos e maiúsculas, vírgula como "qualquer um"); `plan(rules, facts) -> Vec<Planned>` com ordem, "parar" e conflitos.
- **`MessageFacts`:** o que já vem do cabeçalho e da prévia; acrescentar `LIST-ID` e `LIST-UNSUBSCRIBE` ao `HEADER_QUERY` (guardados em `message` por uma coluna nova) para "Lista de e-mails" e o modelo Newsletters.
- **Sincronização:** em `run_folder_sync`, depois de `sync_folder` e antes de `notify::after_inbox_sync`, `rules::apply(state, account, folder, new_ids)`; devolve os ids que continuam para as notificações e os que pedem "Notificar sempre".
- **Ações:** mover, arquivar e excluir por `store::create_pending_moves` e `moves::schedule` (com o mesmo lote para o desfazer); flags por `set_flags`; sinalizar por `set_followup`; marcador por `UID COPY`.
- **Comandos:** `list_rules`, `save_rule`, `delete_rule`, `reorder_rules`, `preview_rule` (casa a regra com os últimos 500 do banco, sem rede), `run_rule_now`, `rule_history`, `undo_rule_run`, `rule_from_message` (sugestões).
- **Limpeza (CA8):** tarefa diária no mesmo laço da sincronização periódica.

### 6. Decisões

- **D1 (aprovada em 26 out 2026: por conta):** regras por conta ou valendo para todas. Pastas mudam de conta para conta; uma regra "todas as contas" só faria sentido para ações sem pasta (lida, estrela, notificação). Recomendo por conta, com "Duplicar para outra conta".
- **D2 (aprovada em 26 out 2026: Caixa de entrada e, opcionalmente, outras pastas):** onde as regras rodam ao sincronizar. Spam, Lixeira, Enviados e Rascunhos nunca.
- **D3 (aprovada em 26 out 2026: só o trecho):** condição sobre o texto. O trecho (até 200 caracteres) já vem com a prévia; o texto inteiro exigiria baixar o corpo de cada e-mail antes de aplicar a regra (mais lento, e mais dados de rede).
- **D4 (aprovada em 26 out 2026: deixar para depois):** condição sobre um cabeçalho qualquer, num modo avançado.
- **D5 (aprovada em 26 out 2026: não nesta story):** encaminhar e responder automaticamente. Mandam e-mail em nome do usuário sem ele ver, e um erro de regra pode vazar mensagens. Se vierem, numa story própria, com lista de destinatários permitidos.
- **D6 (aprovada em 26 out 2026: sim):** "Notificar sempre" fura o Não perturbe e o horário de silêncio.
- **D7 (aprovada em 26 out 2026: não agora):** criar as regras também no servidor (Sieve/ManageSieve ou a API do provedor), para valerem com o app fechado. Depende de cada provedor; fica registrado para depois.
- **D8 (aprovada em 26 out 2026: 30 dias):** quanto tempo o histórico guarda os lotes (e o desfazer).

### 7. Riscos técnicos

- **App fechado:** as regras só valem com o Epistula aberto (inclusive na bandeja). Fica claro na tela de regras.
- **Contas com muita mensagem:** "Rodar agora" e a limpeza podem mexer em milhares de e-mails; vão em pacotes, com progresso e desfazer.
- **Laços:** uma regra que move para uma pasta onde outra regra move de volta. Regras não rodam de novo sobre o que moveram, e o editor recusa origem igual ao destino.
- **Conflito com o mover manual:** a mensagem que o usuário está movendo (pendente) é ignorada pelas regras.
- **Marcadores:** copiar para pasta duplica a mensagem em servidores sem marcadores; o texto da ação diz isso nesses casos.

### 8. Implementação (26 out 2026)

- **Backend:** migração 0007 (`rule`, `rule_run`, `rule_hit`, `message.list_id`); `rules.rs` (motor puro: condições, "todas"/"qualquer uma", exceções, ordem, "parar", a primeira ação de pasta vence, regra encoberta); `rule_runner.rs` (aplicar ao sincronizar, prévia, rodar agora, limpeza diária, desfazer, sugestões); `LIST-ID` e `LIST-UNSUBSCRIBE` no `HEADER_QUERY`; `notify::after_inbox_sync` recebe o que as regras silenciaram e o que avisa sempre; `notify::bell` para os lotes grandes e os erros.
- **Ao sincronizar:** a primeira sincronização de cada pasta não passa pelas regras (CA3). Mover, arquivar e excluir entram na fila do mover com prazo imediato (sem os 6 s do "Desfazer" manual, porque o desfazer é pelo histórico).
- **Desfazer (CA6):** o que ainda não saiu da origem tem o movimento cancelado; o que já está no destino volta por um movimento novo; o que foi arquivado (está só em "Todos os e-mails") volta por cópia para a origem, porque sair de "Todos" apagaria a mensagem no servidor; lida e estrela voltam ao que eram. Marcadores (cópias) não são desfeitos.
- **Frontend:** Configurações → Regras (`features/rules/`): lista com a frase (L58, L59), arrastar ou Alt+↑/↓, menu (Editar, Rodar agora, Duplicar, Subir, Descer, Excluir), modelos no estado vazio e em "Nova regra" (L62), editor com prévia ao vivo e aviso de regra encoberta (L60, L61), histórico com desfazer (L63). Barra de comandos: "Mais" → "Criar regra…", "Sempre mover de…", "Excluir tudo de…" (CA5). Toast "Regras organizaram N e-mails" com "Ver" e "Desfazer".
- **Simplificações:** o editor escolhe no máximo uma pasta extra além da Caixa de entrada (o formato aceita várias); "Duplicar" para outra conta abre o editor com as pastas a escolher de novo; "Rodar agora" mostra o resultado no fim, sem barra de progresso.
- **Testes:** 155 no Rust e 78 no frontend passam; clippy, typecheck e build sem avisos.
- **Falta (manual, com o servidor real):** regra agindo numa sincronização, "Notificar sempre" no Não perturbe, desfazer de um arquivamento e a limpeza diária.

### 9. Fora do escopo

- Encaminhar e responder automaticamente (D5).
- Regras no servidor (D7).
- Expressões regulares e cabeçalhos quaisquer (D4).
- Classificação automática por inteligência artificial.

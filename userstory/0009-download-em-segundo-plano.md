## User Story 9: E-mails prontos para ler, baixados em segundo plano

**Como** usuário do Epistula,
**quero** que a lista apareça logo na primeira sincronização e que o conteúdo dos e-mails já esteja no computador quando eu abrir,
**para que** ler seja instantâneo, sem esperar o download a cada mensagem.

---

### 1. Contexto

Pedido em 25 out 2026. Hoje:

- A primeira sincronização baixa os cabeçalhos das 2000 mensagens mais recentes de cada pasta (D5 da US3) de uma vez; a lista só aparece no fim.
- O corpo de cada mensagem só é baixado na primeira vez que ela é aberta (depois fica no banco cifrado e abre sem rede). Com 2000 mensagens, quase toda abertura espera a rede.
- A conversa em cards (US8) abre várias mensagens de uma vez, e cada uma faz o próprio download.
- **Já corrigido em 25 out 2026, antes desta story:** abrir uma mensagem não lida que já estava salva esperava o servidor confirmar o "lida" antes de mostrar. Agora ela aparece na hora e o "lida" vai ao servidor em seguida (se falhar, a próxima sincronização traz o estado do servidor).

### 2. Regra do design system (obrigatória)

Toda peça nova de interface passaria primeiro por um rascunho. A opção em Configurações → E-mail usa só peças que o DS já tem (`.settings__group`, `.setting`, `.select`, como em modais.html), então não houve rascunho.

### 3. Critérios de aceite

**CA1. Lista rápida na primeira sincronização**

- Na primeira sincronização de uma pasta, vêm primeiro os cabeçalhos das 200 mais recentes; a lista aparece assim que eles chegam.
- O resto, até 2000, chega em pacotes de 250, do mais novo para o mais antigo, sem travar a lista nem a leitura. A lista cresce sozinha (o contador e a paginação acompanham).
- Uma sincronização interrompida (app fechado, sem conexão) continua de onde parou.

**CA2. Conteúdo baixado em segundo plano**

- Depois de cada sincronização, uma fila de baixa prioridade baixa o conteúdo das mensagens ainda não salvas, das mais novas para as mais antigas, em pacotes de cerca de 25.
- Ordem: Caixa de entrada; depois Enviados; depois as outras pastas na ordem da coluna de pastas. Lixeira e Spam ficam de fora.
- Abrir uma mensagem sempre passa na frente da fila.
- Baixar em segundo plano não marca como lida (`BODY.PEEK`).
- Entram só as mensagens dentro do período escolhido (CA4) e até o limite de tamanho (D2). As outras continuam baixando ao abrir.
- A fila pausa sem conexão e com a conta precisando reconectar, e retoma sozinha.

**CA3. Baixar antes de chegar**

- Ao abrir uma mensagem, o app já busca o conteúdo das 3 seguintes da lista e das outras mensagens da conversa.
- Ao parar o mouse sobre uma linha por meio segundo, busca aquela.

**CA4. Escolha do usuário (Configurações → E-mail)**

- "Baixar e-mails para ler sem conexão": Últimos 30 dias, Últimos 90 dias, Tudo (dentro das 2000 por pasta) ou Só ao abrir.
- Mostra quanto espaço os e-mails salvos ocupam neste computador.
- Trocar para um período menor não apaga nada na hora (D4).

### 4. Backend

- **Fila de download:** tarefa por conta numa faixa de conexão de baixa prioridade (`Lane::Sync` ou uma nova), com `UID FETCH <uids> (BODY.PEEK[] RFC822.SIZE)` em pacotes; o resultado passa pelo mesmo caminho do corpo aberto (`message_body`, anexos, prévia).
- **Prioridade:** o pedido de abrir (`get_message`, `Lane::Interactive`) não espera a fila.
- **Primeira sincronização em etapas:** `initial_window` passa a ser feita em pacotes (200, depois 250 por vez), salvando o progresso para continuar depois.
- **Pré-busca:** comando `prefetch_messages { messageIds }`, chamado pelo frontend ao abrir uma mensagem e ao parar o mouse numa linha.
- **Ajuste:** `app_setting` com o período (`offline_days`: 30, 90, `all`, `none`).
- **Logs:** só quantidades e tempos, nunca assunto, remetente ou conteúdo.

### 5. Decisões

- **D1 (aprovada em 25 out 2026: 30 dias):** período padrão do download em segundo plano.
- **D2 (aprovada em 25 out 2026: 2 MB):** tamanho máximo de uma mensagem baixada em segundo plano. Acima disso (quase sempre anexos grandes), só ao abrir.
- **D3 (aprovada em 25 out 2026: 200 e depois 250):** quantos cabeçalhos vêm primeiro e o tamanho dos pacotes seguintes.
- **D4 (aprovada em 25 out 2026: não apagar):** ao escolher um período menor, os e-mails já salvos fora dele continuam no computador. A alternativa é apagar os corpos antigos para liberar espaço.
- **D5 (aprovada em 25 out 2026: não agora):** baixar os anexos separados do texto (só o texto em segundo plano). Economiza espaço, mas exige guardar as partes da mensagem separadamente.

### 6. Riscos técnicos

- **Espaço em disco:** uma mensagem tem em média 50 a 100 KB com anexos; 2000 por pasta podem passar de 100 MB. Por isso o limite de período e de tamanho.
- **Limites do servidor:** muitos downloads seguidos podem ser limitados pelo provedor. A fila usa pacotes pequenos, uma conexão por conta e espera um pouco ao receber erro de limite.
- **Concorrência com a sincronização:** a fila e a sincronização usam o mesmo banco; gravar corpo e cabeçalho da mesma mensagem precisa continuar consistente (o mesmo `message_id`).

### 7. Implementação (25 out 2026)

- `sync.rs`: a primeira leitura traz `FIRST_HEADERS` (200); `backfill` completa até 2000 em pacotes de `BACKFILL_BATCH` (250), avisando a lista a cada pacote e continuando de onde parou (conta o que já está salvo).
- `offline.rs`: fila por conta (`Downloader`, acordada ao fim de cada sincronização e a cada 2 min), lotes de 25 com `BODY.PEEK[]` na conexão de sincronização, até 2 MB e no período escolhido; `prefetch` para o comando `prefetch_messages`.
- `store.rs`: `bodies_to_download` (ordem Caixa de entrada, Enviados, outras, "Todos"; sem Lixeira e Spam), `bodies_for_keys`, `offline_days`.
- Frontend: ao abrir, as 3 seguintes da lista; na conversa, as recolhidas; meio segundo com o mouse numa linha; Configurações → E-mail → "Ler sem conexão".
- Falta o teste manual com o servidor real (tempo da primeira sincronização e uso de disco).

### 8. Fora do escopo

- Busca no conteúdo completo dos e-mails (só cabeçalhos e prévia, como hoje).
- Download de anexos para abrir ou salvar.

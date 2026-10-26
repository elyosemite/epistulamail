## User Story 8: Notificações, bandeja do sistema e conversas em cards

**Como** usuário do Epistula,
**quero** ser avisado do que importa (e-mails novos, respostas aos meus e-mails, prazos, falhas e contas que precisam de atenção) mesmo com a janela fechada, e ler as respostas de uma conversa separadas em cards,
**para que** eu não perca nada e entenda uma conversa longa sem garimpar um texto único.

---

### 1. Contexto

Continuação da US7. Duas partes:

- **Notificações** (os quatro grupos pedidos): mensagens chegando, o que o usuário pediu ao app, saúde da conta e os controles. O app passa a continuar na bandeja do sistema ao fechar a janela (decidido pelo usuário em 25 out 2026); sem isso, ninguém é avisado com a janela fechada. Não há servidor do Epistula, então os avisos dependem do app estar rodando (janela aberta, minimizada ou na bandeja).
- **Conversas:** hoje a leitura mostra uma mensagem por vez, e as respostas anteriores aparecem dentro do texto, como citação ("> …", "Em …, escreveu:"). Fica difícil de ler. A leitura passa a mostrar a conversa como uma pilha de cards, com os cards de mensagem que o DS já tem.

### 2. Regra do design system (obrigatória)

Vale a seção 2 da US1. As peças novas estão no rascunho `design-system/rascunho-us8.html`, que precisa ser aprovado antes de entrar no DS e no app.

### 3. Lacunas no design system

| # | Lacuna | Proposta |
|---|---|---|
| L46 | Conversa em cards | `.conversation`: pilha de `.message--collapsed` (já existe: uma linha com avatar, remetente, trecho e hora) e `.message.message--card` (a mensagem aberta num card com borda). `.conversation__more` junta as do meio ("3 mensagens anteriores") |
| L47 | Texto citado dentro da mensagem | `.message__quote-toggle`: o botão "…" (o mesmo da resposta da US7) esconde a parte citada e a assinatura repetida de cada mensagem |
| L48 | Conversa na lista | `.thread__count` (já existe) ao lado do remetente: quantas mensagens a conversa tem. Os remetentes viram "Marina, Rafael, você" |
| L49 | Configurações → Notificações | Seção nova com os `.setting`, `.toggle`, `.select`, `.segmented` e `.setting-list` que já existem |
| L50 | Prévia das notificações do Windows | Só documentação: o Windows desenha a notificação; o DS mostra o texto, a ordem das linhas e o ícone de cada tipo |
| L51 | Menu da bandeja | Só documentação: o menu é nativo do Windows; o DS lista os itens e a ordem |
| L52 | Sino no header | Botão `.bell` ao lado da foto, com `.badge--primary` (`.bell__count`) das não vistas, e o painel `.notice-panel` com as notificações recentes (`.notice`, `.notice.is-unread`), "Marcar todas como vistas", "Não perturbe por 1 hora" e o atalho para Configurações |
| — | Ícones novos | `bell` e `bell-off` |

### 4. Critérios de aceite

**CA1. Mensagens chegando (grupo 1)**

- **E-mail novo** na Caixa de entrada de uma conta ativa, chegado depois da última sincronização: uma notificação com conta, remetente, assunto e o começo do texto.
- Várias de uma vez (mais de 3 na mesma sincronização) viram uma só: "5 e-mails novos em Trabalho", com "Marina Couto, Rafael Dias e mais 3" (D4).
- **Resposta a um e-mail seu:** a mensagem nova cujo `In-Reply-To` ou `References` aponta para um e-mail enviado por você ganha uma notificação própria, "Marina Couto respondeu", mesmo quando os e-mails novos em geral estão desligados.
- Só a Caixa de entrada notifica (D10). Spam, Lixeira e marcadores, não. A primeira sincronização de uma conta e a primeira depois de muito tempo fechado não notificam o que já existia (só um resumo: "12 e-mails novos em Trabalho enquanto o Epistula estava fechado").

**CA2. O que o usuário pediu ao app (grupo 2)**

- **Prazo da sinalização:** às 9h (ajustável), um aviso "3 sinalizadas vencem hoje" com a primeira pelo assunto. Atrasadas entram no mesmo aviso.
- **Envio que falhou** com a janela fora de foco: "A mensagem não foi enviada", com o motivo. O texto continua no rascunho.
- **Mover que o servidor recusou:** "2 conversas voltaram para Caixa de entrada".

**CA3. Saúde da conta (grupo 3)**

- **Conta precisa reconectar:** "Reconecte Trabalho para continuar recebendo e-mails", uma vez por perda de acesso.
- **Sem conexão há mais de 1 hora** (desligado por padrão, D7): "Sem conexão há 1 hora. Os e-mails novos aparecem quando a conexão voltar."

**CA4. Controles (grupo 4)**

- Configurações ganha a seção **Notificações**:
  - **Avisar quando**: um interruptor por tipo (e-mails novos, respostas aos seus e-mails, prazos com o horário, envio ou mover que falhou, conta que precisa reconectar, sem conexão).
  - **Contas**: ligar e desligar por conta.
  - **O que aparece na notificação**: "Remetente e assunto" ou "Só a conta" ("Novo e-mail em Trabalho"), D5. Notificações aparecem na tela bloqueada e na Central de Notificações, fora do banco cifrado.
  - **Não perturbe**: interruptor e horário (padrão das 22h às 7h). Durante o horário nada aparece; ao terminar, um resumo do que chegou.
  - **Ao fechar a janela**: "Continuar na bandeja do sistema" (ligado por padrão) e "Abrir o Epistula ao iniciar o Windows" (desligado por padrão, D8).
- **Clicar na notificação** abre o app (traz a janela de volta da bandeja), na conta e na mensagem certas. O resumo abre a Caixa de entrada da conta.
- **Contador na barra de tarefas:** o ícone mostra que há não lidas (D9). Some quando não há.
- As notificações não têm botões de ação, como marcar como lida ou arquivar (D11).

**CA5. Bandeja do sistema**

- Fechar a janela (X) esconde o app na bandeja. Na primeira vez, aparece um aviso: "O Epistula continua na bandeja do sistema para avisar sobre e-mails novos. Para sair, use Sair no ícone da bandeja."
- Clique no ícone da bandeja: mostra a janela. Menu do ícone: **Abrir o Epistula**, **Novo e-mail**, **Sincronizar agora**, separador, **Não perturbe por 1 hora** (ou **Retomar notificações**), separador, **Sair do Epistula**.
- O tooltip do ícone diz "Epistula" e, com não lidas, "Epistula · 12 não lidas".
- **Sair do Epistula** encerra de verdade (sincronização e movers pendentes terminam como hoje ao fechar).

**CA6. Conversas em cards**

- A leitura mostra a conversa inteira da mensagem aberta, da mais antiga para a mais nova, uma mensagem por card:
  - **Abertas:** a mais recente e as não lidas, num card com cabeçalho (remetente, destinatários, hora), corpo e anexos.
  - **Recolhidas:** as demais, numa linha cada (avatar, remetente, começo do texto, hora). Clicar abre o card.
  - Com mais de 4 mensagens recolhidas seguidas, as do meio viram uma linha "3 mensagens anteriores", que abre todas.
- Em cada card aberto, a parte citada (o que a mensagem repete das anteriores) fica escondida no botão "…" (D3). Assim cada card mostra só o que aquela pessoa escreveu.
- As mensagens que você enviou entram na conversa como "Você".
- Responder, Responder a todos e Encaminhar agem sobre a mensagem do card (cada card aberto tem os botões). Os comandos do header agem sobre a mais recente.
- **Lista por conversa (D1):** cada conversa aparece uma vez na lista, na data da mensagem mais recente, com a quantidade (`.thread__count`) e os participantes. Ela fica em negrito se alguma mensagem não foi lida.

**CA7. Sino no header (pedido em 25 out 2026, depois da aprovação do primeiro rascunho)**

- Um botão com o ícone de sino fica no fim do header, entre os botões das colunas e a foto da conta. Atalho `Alt+N`.
- O número sobre o sino mostra quantas notificações você ainda não viu (até "9+"). Sem nenhuma, só o sino. Em Não perturbe, o ícone vira o sino riscado.
- Clicar abre um painel com as notificações dos últimos 7 dias, de todas as contas, da mais nova para a mais antiga, com o mesmo texto que saiu no Windows. Entram também as guardadas durante o Não perturbe. As ainda não vistas têm o ponto azul.
- Abrir o painel marca todas como vistas. "Marcar todas como vistas" faz o mesmo sem abrir nada.
- Clicar numa notificação leva à conta e à mensagem, como clicar na notificação do Windows.
- Embaixo do painel: "Não perturbe por 1 hora" (o mesmo da bandeja) e "Configurações", que abre Configurações → Notificações.
- Sem notificações: "Nada novo por aqui".
- Com a privacidade "Só a conta", o painel também não mostra remetente nem assunto.

### 5. Backend

- **Conversa:** agrupada pelos cabeçalhos `Message-ID`, `In-Reply-To` e `References` (a raiz da cadeia vira o `thread_id`), buscados junto com os cabeçalhos. Mudança em relação ao rascunho: o `X-GM-THRID` do servidor não é exposto pela biblioteca IMAP usada, e os cabeçalhos padrão valem para qualquer provedor. As mensagens salvas antes da US8 ganham a conversa aos poucos, a cada sincronização. Migração com `message.thread_id` e índice. Comando `get_conversation { messageId }` → mensagens da conversa (cabeçalhos, flags, se é sua), de todas as pastas da conta, sem repetir; o corpo de cada uma continua vindo de `get_message`.
- **Lista por conversa:** `list_messages` agrupa por `thread_id` na pasta (a mais recente de cada conversa, com quantidade e participantes).
- **Texto citado:** no corpo de texto puro, as linhas com `>` e o bloco "Em …, … escreveu:"; no HTML, os blocos `.gmail_quote`, `blockquote[type=cite]` e equivalentes, marcados pelo sanitizador para o frontend recolher.
- **Notificações:** módulo `notify` alimentado pela sincronização (UIDs novos da Caixa de entrada), pelo agendador (prazos, 1 vez por dia no horário escolhido), pelos eventos de envio e mover, e pela mudança de status da conta. Notificação nativa do Windows com o nome e o ícone do app e retorno do clique (para abrir na mensagem); o texto é montado no Rust e respeita a privacidade escolhida.
- **Resposta a você:** `In-Reply-To`/`References` comparados com os `Message-ID` das mensagens da pasta Enviados (e dos enviados pelo app).
- **Bandeja:** ícone com menu (recurso `tray-icon` do Tauri); fechar a janela a esconde; "Sair" encerra. Contador na barra de tarefas com o ícone sobreposto do Windows.
- **Histórico do sino:** tabela `notice` no banco cifrado (título, texto, destino, hora, vista), com as dos últimos 7 dias; comandos `list_notices`, `mark_notices_seen` e o evento `notices-updated` para o contador.
- **Ajustes:** `app_setting` com os tipos ligados, privacidade, Não perturbe, horário dos prazos, bandeja e iniciar com o Windows; `account_setting` com notificações por conta.
- **Logs:** nunca remetente, assunto ou trecho de uma notificação. Só tipo, quantidade e conta como id interno.

### 6. Decisões

- **D1 (aprovada):** a lista passa a ter uma linha por conversa, como no Gmail. A alternativa é manter uma linha por mensagem e mostrar a conversa só na leitura.
- **D2 (aprovada):** na conversa, ficam abertas a mais recente e as não lidas; o resto fica recolhido, e com mais de 4 recolhidas seguidas, as do meio viram "N mensagens anteriores".
- **D3 (aprovada):** esconder a parte citada de cada mensagem no botão "…".
- **D4 (aprovada):** até 3 e-mails novos na mesma sincronização, uma notificação cada; acima disso, um resumo por conta.
- **D5 (aprovada: "Remetente e assunto" como padrão):** o que aparece na notificação. "Só a conta" fica como opção para quem divide a tela ou deixa o computador desbloqueado.
- **D6 (aprovada: 9h):** horário do aviso diário dos prazos, ajustável.
- **D7 (aprovada: desligado):** aviso de "sem conexão há mais de 1 hora".
- **D8 (aprovada: incluir, desligado por padrão):** "Abrir o Epistula ao iniciar o Windows", minimizado na bandeja. Sem isso, depois de reiniciar o computador ninguém é avisado até o app ser aberto.
- **D9 (aprovada: um ponto):** o contador da barra de tarefas. O Windows mostra um ícone pequeno sobre o do app; um ponto azul é simples e legível, um número exige desenhar a imagem a cada mudança. A quantidade fica no tooltip da bandeja.
- **D10 (aprovada: só a Caixa de entrada):** quais pastas notificam.
- **D12 (aprovada):** o sino guarda as notificações dos últimos 7 dias, e abrir o painel marca todas como vistas.
- **D11 (aprovada: não agora):** botões na notificação (Marcar como lida, Arquivar). O clique para abrir já resolve o principal.

### 7. Riscos técnicos

- **Identidade do app no Windows:** a notificação mostra o nome e o ícone do app e devolve o clique só para um app instalado (atalho no Menu Iniciar com AppUserModelID, criado pelo instalador). No `pnpm tauri dev`, ela pode aparecer com outro nome e sem o retorno do clique. O teste final é com o app instalado.
- **Retorno do clique:** o plugin padrão de notificação do Tauri não entrega o clique no Windows. A proposta é usar a API de notificações do Windows diretamente (a mesma biblioteca que o plugin usa por baixo), com o retorno do clique.

### 8. Fora do escopo

- Notificar com o app totalmente encerrado (precisaria de um servidor ou do serviço de push do Windows).
- VIP, menções e regras por remetente.
- Ações dentro da notificação (D11).

## User Story 7: Envio de e-mail, menu da conta, densidade da lista, colunas recolhíveis e correção do mover

**Como** usuário do Epistula,
**quero** enviar e-mails (novos, respostas, respostas a todos e encaminhamentos), ter minha conta e as preferências rápidas num menu na minha foto, escolher a densidade da lista, esconder as colunas que não estou usando e ver o mover acontecer na hora,
**para que** eu use o Epistula no dia a dia sem voltar ao Gmail e com o foco no que estou lendo.

---

### 1. Contexto

Continuação da US6. A US4 deixou a interface de resposta pronta, com rascunho local, e o botão **Enviar** desabilitado (D3 da US4). Esta story liga o envio. Ela também corrige dois defeitos do mover, relatados por usuários (seção 3).

### 2. Regra do design system (obrigatória)

Vale a seção 2 da US1. As peças novas passaram pelo rascunho `design-system/rascunho-us7.html`, aprovado em 25 out 2026, incorporado ao DS e apagado (seção 10).

### 3. Diagnóstico: mover para uma pasta

**Defeito A.** O e-mail movido volta para a lista de origem, e ao clicar nele aparece "Esta mensagem não está mais nesta pasta".

- Ao mover, a lista tira o e-mail na hora, mas o servidor só recebe o pedido depois dos 6 segundos do "Desfazer" (`lib/undo.ts`, `UNDO_MS`). Até lá, o e-mail continua na pasta de origem no banco local.
- Se a lista recarregar nesses 6 segundos, o e-mail volta. Isso acontece com a sincronização periódica (evento `mailbox-updated`), com a troca de pasta e retorno, ou com a paginação.
- Depois disso, o mover é feito no servidor e o e-mail sai do banco, mas a lista não recarrega, porque acha que já o tinha tirado. A linha fica ali, e ao abri-la o backend responde `MessageNotFound`.

**Defeito B.** O e-mail demora a aparecer na pasta de destino.

- O destino só ganha o e-mail depois dos 6 segundos do "Desfazer" e de uma sincronização IMAP completa daquela pasta (`sync::spawn_folder_sync`). Se o usuário abre o destino antes disso, o e-mail não está lá. Ele só aparece no próximo `mailbox-updated`.

**Correção proposta (D1):** o mover passa a ser registrado no banco local na hora (tabela `pending_move`), e o backend passa a cuidar do prazo do "Desfazer":

1. Ao mover, o backend grava o movimento pendente e responde na hora. As consultas de lista, contagem, busca e sinalizadas já tratam o e-mail como estando no destino: ele some da origem e aparece no destino, com a mesma data e as mesmas marcas.
2. O "Desfazer" apaga o registro, e o e-mail volta para a origem no banco. Nada foi ao servidor.
3. Passados 6 segundos, o backend envia o `MOVE` ao servidor e sincroniza o destino. O e-mail passa a ter o UID do destino, e a lista recebe um `mailbox-updated` para trocar o id provisório pelo definitivo. A mensagem aberta continua aberta.
4. Se o servidor recusar ou a conexão cair, o registro é apagado, o e-mail volta para a origem e aparece o toast de falha (como hoje).
5. Se o app fechar antes dos 6 segundos, o movimento pendente é enviado na próxima abertura, porque o usuário não desfez.

Vale para mover, arrastar, arquivar e excluir (Lixeira). "Excluir de vez" na Lixeira continua com confirmação e sem "Desfazer".

### 4. Lacunas no design system

| # | Lacuna | Proposta |
|---|---|---|
| L40 | Menu da conta | `.account-menu`: menu ancorado na foto, com três seções separadas por `.menu__sep`: (1) foto, nome e e-mail; (2) Configurações, Tema e Densidade; (3) Sair |
| L41 | Controle segmentado | `.segmented`, hoje só no `docs.css`, vira componente (Tema: Claro/Escuro/Sistema; Densidade: três níveis) com `role="radiogroup"` |
| L42 | Fim do header | `.appbar__end`: na extrema direita, os dois botões de colunas e, por último, o botão da foto da conta. A marca (ícone + Epistula) continua na extrema esquerda e a busca no meio |
| L43 | Colunas recolhíveis | `.app-shell--no-folders` e `.app-shell--no-reader` na casca em blocos; com as duas, só a lista fica, na largura toda. Ícones `sidebar-left` e `sidebar-right` |
| L44 | Densidade da lista | Três níveis. **Confortável:** remetente, assunto, um trecho do texto e ícones de anexo e imagem (`.thread__icons`). **Padrão:** remetente e assunto, como hoje. **Compacta:** remetente e assunto na mesma linha, em duas colunas (`.thread-list--compact`, que já existe) |
| L45 | Envio | **Enviar** ativo; estado "Enviando…" no `.compose`; toast "Mensagem enviada" com **Desfazer** durante o prazo; anexos do encaminhamento como `.attachment` com remover; citação da resposta recolhida (`.compose__quote-toggle`) |

### 5. Critérios de aceite

**CA1. Enviar um e-mail novo**

- **Novo e-mail** abre o card. **Enviar** fica ativo com pelo menos um destinatário válido em Para, Cc ou Cco.
- Um destinatário inválido (chip vermelho) bloqueia o envio, e o motivo aparece junto do botão.
- Sem assunto, o app pergunta "Enviar sem assunto?".
- Ao enviar, o card fecha e aparece o toast "Enviando…" com **Desfazer** por 6 segundos (D3). Depois do envio, aparece "Mensagem enviada", o rascunho é apagado e o e-mail aparece em **Enviados** depois da sincronização.
- Se falhar (sem conexão, recusa do servidor ou acesso perdido), o card reabre com o texto intacto, o rascunho continua salvo e uma mensagem explica o motivo. O app não guarda uma fila de envio para depois (D4).
- `Ctrl+Enter` envia.

**CA2. Responder e responder a todos**

- Os destinatários e o assunto continuam como na US4.
- A mensagem enviada leva `In-Reply-To` e `References`, para o Gmail e os outros clientes a agruparem na mesma conversa.
- Abaixo do texto vai a citação da mensagem original: "Em 23 de out. de 2026, às 10:42, Marina Couto <marina@lumen.com.br> escreveu:", seguida do texto com `> ` (D5). No card, a citação aparece recolhida num botão "…", que pode ser expandido e apagado.

**CA3. Encaminhar**

- O encaminhamento leva o texto e os anexos da mensagem original (D6). Os anexos aparecem no card e podem ser removidos um a um.
- Acima de 25 MB no total, o envio é bloqueado com a mensagem "Os anexos passam do limite de 25 MB".

**CA4. Configurações abrem em Geral**

- Ao abrir as Configurações (botão do menu da conta ou `Ctrl+,`), a seção inicial é **Geral**.
- Geral passa a ter as opções de lista: Densidade e Colunas visíveis (D7). Assim a seção deixa de ser "Em breve".

**CA5. Mover (correção, seção 3)**

- O e-mail movido some da origem na hora e não volta, com qualquer recarga da lista, troca de pasta ou sincronização.
- Ao abrir a pasta de destino logo depois de mover, o e-mail já está lá.
- **Desfazer** devolve o e-mail para a origem, na mesma posição.
- Clicar num e-mail da lista nunca mostra "Esta mensagem não está mais nesta pasta" por causa de um mover feito neste computador.

**CA6. Menu da conta**

- O header fica assim: ícone + Epistula na extrema esquerda, a busca no meio e, na extrema direita, os dois botões de colunas e o botão com a foto da conta ativa (o último). Clicar na foto, ou usar `Alt+A`, abre o menu, alinhado à direita da foto.
- **Seção 1:** foto, nome e e-mail abaixo. Também ficam aqui as informações da seção 7, quando aprovadas.
- **Seção 2:** **Configurações** (`Ctrl+,`), **Tema** (Claro, Escuro, Sistema) e **Densidade** (Confortável, Padrão, Compacta). Tema e densidade valem na hora e ficam salvos.
- **Seção 3:** **Sair**, que abre a confirmação de sair que já existe.
- Nenhum texto do menu (nem das mensagens de envio) cita o provedor da conta, como Gmail ou Google. O endereço de e-mail aparece como é.
- O botão de configurações sai de perto da busca, e o "nome · e-mail" sai do header: os dois vão para o menu.
- `Esc` fecha o menu e devolve o foco à foto. As setas percorrem os itens.

**CA7. Densidade da lista**

- **Confortável:** remetente, data, assunto, um trecho do início do texto e, à direita do assunto, um ícone de clipe quando há anexos e um ícone de imagem quando há imagens anexadas.
- **Padrão:** remetente, data e assunto. É o visual de hoje e continua sendo o padrão.
- **Compacta:** uma linha, com o remetente numa coluna e o assunto na outra.
- A densidade vale para todas as contas e fica salva no banco (como o tema, D3 da US6).

**CA8. Colunas recolhíveis**

- Dois botões, logo à esquerda da foto: **Mostrar/esconder pastas** (`Ctrl+B`) e **Mostrar/esconder leitura** (`Ctrl+Alt+B`). O tooltip mostra o atalho, e `aria-pressed` indica o estado.
- Sem as pastas, a lista começa junto da coluna de contas. Sem a leitura, a lista ocupa o resto da largura e o divisor some. Sem as duas, só a lista fica na tela.
- Abrir uma mensagem, ou começar um e-mail novo, com a leitura escondida mostra a coluna de leitura de novo (D8).
- O estado das duas colunas fica salvo e volta na próxima abertura.
- Os atalhos não disparam com um modal aberto. Também não disparam com o foco num campo de texto, a não ser `Ctrl+B`, que o corpo do e-mail não usa porque ainda é texto simples.

### 6. Backend

- **Envio:** comando `send_message { accountId, kind, inReplyTo?, to, cc, bcc, subject, body, attachments? }`, que devolve um `sendId` e envia depois do prazo do "Desfazer". `cancel_send { sendId }` cancela.
- **SMTP:** `smtp.gmail.com:465` (TLS), autenticação XOAUTH2 com o access token que o app já tem. O escopo `https://mail.google.com/` já cobre o SMTP, então não é preciso reconectar as contas.
- **Montagem da mensagem:** com a crate `lettre` (MIME, cabeçalhos codificados, proteção contra injeção de cabeçalho), sem montagem manual do MIME e sem criptografia própria. O Gmail grava a cópia em Enviados sozinho; o app só sincroniza Enviados depois do envio.
- **Mover pendente:** migração `0004`, com `pending_move (message_id, from_folder_id, from_uid, to_folder_id, due_at)`, consultas de lista e contagem ajustadas e um agendador que executa os vencidos e também os que sobraram da sessão anterior, ao abrir.
- **Trecho e anexos (Confortável, D2):** `message.snippet` (até 200 caracteres de texto puro) e `message.has_images`, preenchidos na sincronização.
- **Preferências:** `app_setting` ganha `density` (`comfortable`, `standard`, `compact`), `show_folders` e `show_reader`. Os comandos `get_appearance` e `set_appearance` passam a incluir esses três campos.
- **Logs:** nada de destinatários, assunto, corpo, trecho ou nomes de anexos. Só contagens, tamanhos e o resultado (`enviado`, `cancelado`, `falhou: <tipo>`).

### 7. Informações sugeridas para o menu da conta (para discutir)

O pedido deixa essas informações em aberto. São sugestões com dados que o app já tem, sem citar o provedor da conta:

1. **Status da sincronização:** "Sincronizado há 2 min", ou "Sem conexão", com o botão **Sincronizar agora**.
2. **Não lidas na conta:** a soma da Caixa de entrada e das outras pastas, por exemplo "12 não lidas".
3. **Neste computador:** quanto o banco local ocupa e quantas mensagens estão salvas para ler sem conexão.
4. **Adicionar outra conta:** atalho para o fluxo de conectar conta.
5. **Atalhos de teclado:** lista dos atalhos do app (`?`).
6. **Versão do Epistula**, no rodapé do menu.

No rascunho, as sugestões 1 a 3 aparecem na seção 1. As sugestões 4 a 6 não aparecem, para manter o menu curto. Diga quais quer e onde. O armazenamento usado no provedor e o atalho para a conta do provedor saíram a pedido (25 out 2026): o menu não cita provedores.

### 8. Decisões

Aprovadas em 25 out 2026 junto com o rascunho ("rascunho aprovado, pode implementar"), seguindo as recomendações.

- **D1 (aprovado: A):** a correção do mover.
  - **A.** Mover pendente no banco, com o prazo do "Desfazer" no backend (seção 3). É instantâneo nas duas pastas e sobrevive a trocar de conta ou fechar o app.
  - **B.** Mover no servidor na hora, com o "Desfazer" movendo de volta. É mais simples, mas o destino ainda espera uma sincronização (cerca de 1 a 2 s), e desfazer custa outra ida ao servidor.
- **D2 (aprovado: A):** de onde vem o trecho do Confortável.
  - **A.** Na sincronização, o app busca o `BODYSTRUCTURE` e os primeiros 2 KB da parte de texto, sem baixar a mensagem inteira. A parte de imagem sai do mesmo `BODYSTRUCTURE`. As mensagens já salvas são completadas aos poucos.
  - **B.** O trecho só aparece nas mensagens já abertas ou baixadas. Não tem custo de rede, mas a maior parte da lista ficaria sem trecho.
- **D3 (aprovado: sim):** "Desfazer envio" por 6 segundos, igual ao do mover.
- **D4 (aprovado: não agora):** fila de envio sem conexão. Por ora, o envio sem conexão falha, avisa e mantém o rascunho. A fila (Caixa de saída) fica para uma story futura.
- **D5 (aprovado: sim):** incluir a citação da mensagem original na resposta, como o Gmail faz.
- **D6 (aprovado: sim):** o encaminhamento leva os anexos da original. Anexar arquivos novos do computador fica para uma story futura.
- **D7 (aprovado: sim):** a seção Geral das Configurações recebe Densidade e Colunas visíveis.
- **D8 (aprovado: A):** abrir uma mensagem com a leitura escondida.
  - **A.** A coluna de leitura volta a aparecer.
  - **B.** A leitura ocupa o lugar da lista, com um botão **Voltar**.
- **D9 (aprovado: Padrão):** o nome do nível do meio da densidade, o visual de hoje.
- **D10 (decidido em 25 out 2026):** ícone + Epistula na extrema esquerda, busca no meio, botões de colunas e foto da conta na extrema direita (o pedido dizia "extrema esquerda" para a foto; corrigido pelo usuário). Com as pastas escondidas, o header deixa de seguir as colunas e fica em linha, na mesma ordem.

### 9. Fora do escopo

- Anexar arquivos do computador, formatação rica (negrito, listas, links), assinatura, envio agendado e fila de envio sem conexão.
- Envio por outros provedores além do Gmail. O catálogo já tem os servidores SMTP, mas a conexão OAuth só existe para o Google.

### 10. Implementação (25 out 2026)

**Design system** (rascunho incorporado e apagado):

- `components.css`: `.segmented` (saiu do `docs.css`, agora com `aria-checked` e `--fill`), `.appbar__end` e `.account-trigger`, `.account-menu` (`__head`, `__name`, `__email`, `__info`, `__line`, `__pref`, `__label`), `.app-shell--no-folders` e `.app-shell--no-reader`, `.thread-list--standard` e `.thread__icons`, `.compose__quote-toggle`, `.compose__attachments`, `.compose__hint` e `.compose.is-sending`.
- Ícones `sidebar-left`, `sidebar-right` e `hard-drive`.
- Páginas: `espacamento.html` (header novo na casca em blocos, interativo, e "Colunas recolhíveis"), `menus.html` (menu da conta), `threads.html` (as três densidades e a demonstração com três opções), `composicao.html` (envio), `modais.html` (Configurações → Geral) e `header.html` (fim do header).

**Backend**

- Mover pendente (D1): migração `0004`, tabela `pending_move`. `move_messages`, `archive_messages` e `delete_messages` (para a Lixeira) registram o lote na hora e devolvem `{ batch, ids, failed }`; `undo_move` desfaz o que ainda não foi ao servidor. O módulo `moves` executa o MOVE 6 s depois (e, ao abrir o app, o que ficou pendente) e avisa falhas pelo evento `moves-finished`. A lista da origem não mostra o que está saindo, a do destino mostra o que está chegando (no topo, com o id da origem) e os contadores das duas pastas acompanham, inclusive depois de uma sincronização. A sincronização do destino encerra o registro quando a mensagem chega com o UID de lá. Abrir uma mensagem já movida no servidor lê da pasta onde ela está; se ainda não chegou, aparece "Esta mensagem está sendo movida".
- Prévia (D2): módulo `preview`. Cada sincronização busca, para até 200 mensagens sem prévia, o `BODYSTRUCTURE` e os primeiros 2 KB da parte de texto (`BODY.PEEK`); o trecho tira citações e assinatura. Abrir uma mensagem também grava a prévia a partir do corpo. A lista ganha `snippet`, `hasFiles` e `hasImages`.
- Envio: módulo `send` com a `lettre` (SMTP com XOAUTH2 e TLS, sem a feature `hostname`). `send_message` valida e agenda (6 s, D3) e `cancel_send` desfaz; o resultado chega pelo evento `send-finished`. Responder leva `In-Reply-To` e `References`; encaminhar leva os anexos escolhidos da original (D6), até 25 MB. Depois do envio, o rascunho é apagado e Enviados sincroniza. O `From` usa o nome da claim `name` (guardado a cada renovação do token). A resposta passa a receber a citação da original no modelo (D5).
- Menu da conta: `account_overview` (última sincronização, não lidas e mensagens salvas, mais o tamanho do banco) e `sync_account`.
- Preferências: `get_appearance` e `set_appearance` incluem `density`, `showFolders` e `showReader`.
- Erros novos: `MessageMoving`, `SmtpConnection`, `SendRejected`, `NoRecipients`, `InvalidRecipient` e `AttachmentsTooLarge`, todos sem citar o provedor.

**Frontend**

- Header: marca, busca e, na extrema direita, `layout/ColumnToggles` e `layout/AccountMenu` (D10). `SettingsButton` e `HeaderAccount` saíram. Atalhos `Ctrl+B`, `Ctrl+Alt+B` e `Alt+A`, desligados com um modal aberto.
- `ui/Segmented` (tema e densidade), `features/settings/GeneralSettings` (D7) e Configurações abrindo em Geral (CA4).
- Densidade: `ThreadList` com a classe da densidade e `ThreadRow` com trecho e ícones; vem de `appearance.density`.
- Colunas: `AppLayout` com `showFolders` e `showReader`, salvos no banco; abrir uma mensagem ou um e-mail novo com a leitura escondida a mostra de novo (D8).
- Mover: o `Mailbox` tira as mensagens da lista e chama o backend na hora; "Desfazer" chama `undo_move`. O `UndoQueue` do frontend saiu.
- Envio: `ComposeCard` com Enviar ativo, motivo do bloqueio, `Ctrl+Enter`, "Enviar sem assunto?", citação recolhida que pode ser removida e anexos do encaminhamento removíveis; `lib/compose.ts` monta o texto enviado. O `Mailbox` fecha o card, mostra "Enviando em 6 s" com Desfazer (reabre o card) e trata `send-finished` ("Mensagem enviada" com "Ver em Enviados", ou o erro com "Abrir rascunho").
- Textos que citavam o Google ou o Gmail ("O Google encerrou o acesso…", "apagada do Gmail") passaram a dizer "servidor de e-mail".

**Diferença do rascunho:** o botão Enviar ficou simples, sem a seta "Opções de envio", porque ainda não há opções (envio agendado está fora do escopo).

**Verificado**

- Rust: **101 testes** (15 novos: mover pendente na origem e no destino, desfazer, execução e encerramento pela sincronização, falha e troca de destino, retomada ao abrir; prévia salva e listada; estrutura MIME e trecho em base64, quoted-printable e HTML cortados; charset limpo; envio sem injeção de cabeçalho e com o Cco só no envelope; encaminhar só os anexos escolhidos; `In-Reply-To`/`References`; preferências de lista; números do menu da conta), estáveis em 3 execuções. `clippy` sem avisos e `cargo deny` ok.
- Frontend: **52 testes** (6 novos: citação e encaminhamento, corpo de cada tipo, validação do Enviar, textos do menu da conta e preferências; saíram os 3 do `UndoQueue`); `tsc` e build ok.
- Interface com backend simulado: menu da conta aberto à direita da foto, densidade Confortável com trecho e ícones, só a lista (Compacta), Configurações abrindo em Geral, resposta com a citação recolhida e expandida, Enviar bloqueado por endereço inválido, card fechando ao enviar com o toast "Enviando em 6 s", arquivar com a lista ajustada e o toast com Desfazer.

**Pendente**

- Teste manual contra o Gmail real: enviar, responder (a conversa precisa ficar agrupada no Gmail), encaminhar com anexos, desfazer o envio, mover e abrir a pasta de destino logo depois, fechar o app no prazo do Desfazer e reabrir.
- O nome do `From` vem da claim `name`, que só chega na próxima renovação do token (ou ao reconectar); até lá, a mensagem sai só com o endereço.

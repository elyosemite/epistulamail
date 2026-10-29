## User Story 20: Outlook pessoal pelo Microsoft Graph

**Como** usuário de uma conta pessoal Outlook.com,
**quero** conectar a minha conta no EpistulaMail sem precisar de um tenant de trabalho/escola,
**para que** eu leia, busque, escreva e organize os meus e-mails no meu computador, com os mesmos cuidados de dados das contas Google.

---

### 1. Contexto

A US19 ligou o Outlook por IMAP e SMTP, mas o registro de aplicativo de uma conta pessoal da Microsoft fica no diretório "Microsoft Accounts", que não tem o service principal do Exchange Online. Sem ele, as permissões `IMAP.AccessAsUser.All` e `SMTP.Send` não podem ser concedidas, e o login termina em `AuthFailed`. O Microsoft Graph é o caminho suportado para contas pessoais, tem service principal no diretório pessoal e cobre Outlook.com, Hotmail e Microsoft 365.

O app passa a usar o Graph para as contas Microsoft. O Gmail continua em IMAP e SMTP. As pastas e mensagens do Graph entram no mesmo banco cifrado, com um UID local sintético e o id remoto ao lado.

### 2. Critérios de aceite

**CA1. Login**

- O Outlook aparece na tela de adicionar conta e abre a página da Microsoft no navegador.
- O app pede só `https://graph.microsoft.com/Mail.ReadWrite`, `https://graph.microsoft.com/Mail.Send`, `offline_access`, `openid` e `email`.
- O redirect é `http://127.0.0.1` numa porta efêmera, com PKCE e `state` de uso único.
- Funciona no diretório pessoal da Microsoft e em Microsoft 365, sem tenant de trabalho/escola para o registro do app.

**CA2. Leitura e envio**

- Pastas, contagens, mensagens, conversas, prévias, busca local, flags, mover, arquivar, excluir e baixar offline funcionam como no Gmail.
- O corpo completo vem em MIME pelo Graph e passa pelo mesmo parser, limpeza de HTML e bloqueio de imagens.
- O envio monta o mesmo MIME da US7 e o entrega pelo Graph, com anexos, rascunhos locais e "Desfazer envio".
- A conversa usa o `conversationId` do Graph.

**CA3. Dados e privacidade**

- O refresh token fica no cofre do sistema, nunca no banco em texto, em arquivo ou no log; com a senha do Epistula ligada, também cifrado por ela.
- Nenhuma mensagem, assunto, endereço ou token passa por um servidor do EpistulaMail. Continua sem telemetria e sem relatório de erro.
- O app não pede contatos, calendário, arquivos, OneDrive nem perfil além do endereço e do nome.
- Sair da conta apaga o token local; a revogação do acesso é feita em `account.live.com/consent/Manage` ou pelo administrador do Microsoft 365.
- A política de privacidade, o modelo de ameaças, os READMEs e as docs falam dos escopos do Graph e dos limites da Microsoft.

**CA4. Linux e Windows**

- O mesmo código Graph roda nos dois sistemas; no Linux, o login usa o navegador do sistema e o token fica no Secret Service.
- O Gmail continua em IMAP/SMTP nos dois sistemas.

**CA5. Testes e documentação**

- Teste de unidade da alocação de UID sintético e do id remoto.
- Clippy e testes verdes no Windows e no Linux.
- Docs em pt, en, es, fr e it e CHANGELOG atualizados.

### 3. Interface

Nenhuma tela nova. O cartão do Outlook continua no seletor de provedores; o fluxo de espera, erro e reconexão é o mesmo. A página "Conectar o Outlook" passa a listar os escopos do Graph.

### 4. Decisões

- **D1. Graph para Microsoft.** O `Provider::Microsoft` passa a usar os escopos do Graph; o Outlook deixa de depender de IMAP/SMTP e de tenant com Exchange Online.
- **D2. Escopos mínimos.** `Mail.ReadWrite`, `Mail.Send`, `offline_access`, `openid` e `email`; nada de contatos, calendário, arquivos ou OneDrive.
- **D3. UID sintético.** O Graph não tem UID IMAP; o app guarda o id remoto em `message_folder.remote_id` e um UID local estável para a paginação. As mensagens mais novas ficam no topo e as páginas antigas descem a partir de `UID_BASE`.
- **D4. Paginação por `@odata.nextLink`.** A primeira página traz as 200 mais recentes; o backfill segue o próximo link até completar a janela. A sincronização periódica não reinicia o link das mensagens antigas.
- **D5. Primeira versão sem delta.** A sincronização atualiza a primeira página e as flags das mensagens nela; não detecta exclusões feitas em outro cliente. O delta do Graph fica para uma próxima história.
- **D6. Sem foto da Microsoft.** A foto do perfil continua sendo do Google; contas Microsoft usam a inicial e a cor.
- **D7. Dados.** Nada muda no cofre, no banco cifrado, na ausência de servidor próprio e nos logs. A Microsoft entra no inventário e na política de privacidade com os escopos do Graph.

### 5. Fora do escopo

- Categorias do Outlook como marcadores.
- Delta sync do Graph e detecção de exclusões remotas.
- Foto do perfil da Microsoft.
- Yahoo, iCloud, Zoho, GMX, AOL e IMAP genérico.

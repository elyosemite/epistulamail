## Fatia 02: Saída SMTP com senha normal

**Parent:** `userstory/0022-configuracao-manual-de-servidores.md`

**Tipo:** AFK

### O que construir

O envio de uma conta manual com servidor SMTP próprio, usuário e senha, usando TLS ou STARTTLS. A conta passa a ler **e** enviar; o teste de saída autentica sem mandar mensagem.

Inclui:

- Uso da configuração de saída salva na fatia 01 (host, porta, segurança, método, usuário, senha).
- "Usar as mesmas credenciais da entrada" e credenciais separadas de saída, com senha própria no cofre.
- SMTP com PLAIN/LOGIN, mantendo XOAUTH2 para as contas OAuth existentes.
- Teste de saída com conexão, EHLO e AUTH, sem enviar mensagem.
- Erros distintos para DNS, TLS, autenticação e recusa do servidor; sem host, usuário ou senha no log.
- Envio real com anexos, Cco, "Desfazer envio" e limites atuais.

### Critérios de aceite

- [ ] A conta manual envia uma mensagem para um servidor SMTP falso em loopback com PLAIN e com LOGIN.
- [ ] "Testar conexão" da saída autentica e encerra a sessão sem enviar e-mail.
- [ ] "Usar as mesmas credenciais" copia usuário e senha; desligado, a saída usa as credenciais próprias.
- [ ] Senha de saída no cofre, `Zeroizing`, sem aparecer no banco, log, evento ou WebView; com a senha do Epistula ligada, cifrada por ela.
- [ ] Falha de autenticação mostra mensagem amigável e marca a conta como `needs_reauth`, mantendo a configuração.
- [ ] Contas Gmail/Outlook continuam enviando por XOAUTH2/Graph sem regressão.
- [ ] Testes de protocolo com SMTP falso cobrem sucesso, senha errada, servidor indisponível e timeout.
- [ ] CHANGELOG, modelo de ameaças e docs (pt/en/es/fr/it) descrevem o envio manual.

### Bloqueada por

- `userstory/0022/01-entrada-imap-senha-normal.md`

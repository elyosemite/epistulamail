## Fatia 05: Senha criptografada (CRAM-MD5) na entrada

**Parent:** `userstory/0022-configuracao-manual-de-servidores.md`

**Tipo:** AFK

### O que construir

O método "Senha criptografada" na entrada IMAP: CRAM-MD5 (RFC 2195), em que o servidor manda um desafio e o app responde com HMAC-MD5 do usuário e da senha, sem mandar a senha em claro. O método só aparece quando o servidor anuncia `AUTH=CRAM-MD5`; sem suporte, erro amigável.

Inclui:

- SASL CRAM-MD5 no cliente IMAP, com a senha vinda do cofre.
- Conferência da capacidade do servidor no teste e na conexão.
- Vetor de teste da RFC 2195 e teste com servidor IMAP falso.
- Tratamento de desafio inválido, resposta recusada e servidor sem CRAM-MD5.
- O envio com CRAM-MD5 continua "Em breve" (D4); a opção de saída não é salva.

### Critérios de aceite

- [ ] Entrada autentica com CRAM-MD5 contra servidor falso que anuncia o mecanismo.
- [ ] Servidor sem `AUTH=CRAM-MD5` recusa a escolha com mensagem que sugere senha normal.
- [ ] A senha não é enviada em claro e não aparece em log, banco, evento ou WebView.
- [ ] Desafio inválido, resposta recusada e timeout têm erros distintos e amigáveis.
- [ ] O vetor `tim tanstaaftanstaaf` da RFC 2195 passa no teste.
- [ ] A UI mostra CRAM-MD5 na entrada e "Em breve" na saída.
- [ ] Modelo de ameaças registra que CRAM-MD5 usa MD5 e que a senha ainda fica no cofre.
- [ ] Docs (pt/en/es/fr/it) e CHANGELOG atualizados.

### Bloqueada por

- `userstory/0022/01-entrada-imap-senha-normal.md`
- `userstory/0022/02-saida-smtp-senha-normal.md`

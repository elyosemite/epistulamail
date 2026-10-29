## Fatia 07: Kerberos/GSSAPI na entrada

**Parent:** `userstory/0022/configuracao-manual-de-servidores.md`

**Tipo:** HITL + AFK

### O que construir

O método Kerberos/GSSAPI na entrada IMAP. A fatia começa por um spike HITL para escolher a estratégia por plataforma e medir o impacto no build, no CI e na release; só depois entra código. O envio com GSSAPI continua "Em breve" (D4).

Inclui:

- Decisão D9: SSPI no Windows e GSSAPI do sistema no Linux (ou alternativa pura Rust), com licença, manutenção, dependências de sistema e impacto nos instaladores.
- SASL GSSAPI no IMAP, com SPN `imap@host`, troca de tokens e confirmação de segurança.
- Mensagens amigáveis para "sem ticket", "servidor não oferece GSSAPI" e falha de contexto.
- Testes de unidade do enquadramento SASL; teste manual com KDC em ambiente controlado.
- Modelo de ameaças e inventário de criptografia atualizados.

### Critérios de aceite

- [ ] A decisão D9 está registrada com a estratégia por plataforma e o impacto no CI/release aprovado.
- [ ] O build continua passando no Windows e no Linux, com as dependências documentadas em `AGENTS.md`/README de build.
- [ ] Entrada autentica com GSSAPI em ambiente de teste com KDC.
- [ ] Servidor sem GSSAPI ou usuário sem ticket recebe erro amigável, sem vazar host ou usuário no log.
- [ ] O enquadramento SASL e a confirmação de segurança têm testes.
- [ ] A UI mostra Kerberos/GSSAPI na entrada e "Em breve" na saída.
- [ ] Modelo de ameaças, docs (pt/en/es/fr/it) e CHANGELOG atualizados.

### Bloqueada por

- `userstory/0022/01-entrada-imap-senha-normal.md`
- `userstory/0022/02-saida-smtp-senha-normal.md`

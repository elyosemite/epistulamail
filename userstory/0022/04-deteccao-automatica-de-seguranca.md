## Fatia 04: Detecção automática de segurança

**Parent:** `userstory/0022-configuracao-manual-de-servidores.md`

**Tipo:** AFK

### O que construir

A opção "Detectar automaticamente" no campo de segurança da entrada e da saída: o app tenta TLS implícito e, se falhar, STARTTLS no mesmo host/porta, sem enviar credenciais. Se nenhum funcionar, a conta não é salva e a mensagem explica o motivo.

Inclui:

- Probe de TLS e STARTTLS para IMAP e SMTP, com timeouts curtos e sem autenticação.
- Gravação da segurança resolvida (`tls` ou `starttls`), nunca "auto".
- Nenhuma tentativa de "sem segurança" (D2).
- Resultado visível no teste de conexão e na edição de servidores.
- Testes de ordem de tentativa, fallback e falha, com servidores falsos.

### Critérios de aceite

- [ ] "Detectar automaticamente" resolve TLS quando o servidor aceita TLS implícito.
- [ ] Sem TLS implícito, resolve STARTTLS quando o servidor anuncia e aceita `STARTTLS`/`STLS`.
- [ ] Sem TLS nem STARTTLS, falha com mensagem amigável e não salva a conta.
- [ ] O probe nunca envia usuário, senha ou comando de autenticação.
- [ ] O certificado é validado em todas as tentativas; certificado inválido não vira "sem TLS".
- [ ] A segurança resolvida é gravada e usada nas conexões seguintes; o formulário mostra o valor resolvido.
- [ ] Testes com servidores falsos cobrem TLS, STARTTLS, ambos, nenhum e timeout.
- [ ] Modelo de ameaças e docs (pt/en/es/fr/it) explicam a detecção e o risco de downgrade.

### Bloqueada por

- `userstory/0022/03-editar-servidores-e-reconectar.md`

## Fatia 06: NTLM na entrada

**Parent:** `userstory/0022-configuracao-manual-de-servidores.md`

**Tipo:** HITL + AFK

### O que construir

O método NTLM na entrada IMAP, no modelo SASL NTLM. A fatia começa por uma decisão HITL sobre o crate e a cobertura por plataforma; só depois entra código. O envio com NTLM continua "Em breve" (D4).

Inclui:

- Decisão D8: crate de NTLM, licença, manutenção, cobertura Windows/Linux e justificativa para o `cargo deny`.
- SASL NTLM no IMAP, com usuário, domínio e senha; mensagens de desafio tratadas em sequência.
- Conferência de `AUTH=NTLM` no servidor; sem suporte, erro amigável.
- Testes com vetores conhecidos e servidor IMAP falso.
- Modelo de ameaças: NTLM é legado, não envia senha em claro, mas tem limitações de segurança e de proxy.

### Critérios de aceite

- [ ] A decisão D8 está registrada com crate, licença, manutenção e matriz de plataformas aprovada.
- [ ] Entrada autentica com NTLM contra servidor falso que anuncia o mecanismo.
- [ ] Servidor sem `AUTH=NTLM` recusa a escolha com mensagem amigável.
- [ ] Usuário com domínio (`DOMINIO\usuario` ou `usuario@dominio`) é tratado corretamente.
- [ ] A senha não aparece em log, banco, evento ou WebView.
- [ ] Testes de desafio, resposta recusada, timeout e servidor sem NTLM.
- [ ] A UI mostra NTLM na entrada e "Em breve" na saída.
- [ ] Modelo de ameaças, docs (pt/en/es/fr/it) e CHANGELOG atualizados.

### Bloqueada por

- `userstory/0022/01-entrada-imap-senha-normal.md`
- `userstory/0022/02-saida-smtp-senha-normal.md`

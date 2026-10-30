## Fatia 08: OAuth2 em conta Gmail manual

**Parent:** US22 (contas manuais, já implementadas no código)

**Tipo:** AFK

### O que construir

Permitir OAuth2 numa conta Gmail configurada manualmente (D3), reaproveitando o cliente OAuth existente, com XOAUTH2 na entrada e na saída. "Outro" não oferece OAuth2; o Outlook continua no Graph.

Inclui:

- Opção OAuth2 no método de entrada/saída apenas quando o provedor é Gmail.
- Reutilização de `provider.rs`/`oauth.rs` para autorização, refresh e revogação.
- XOAUTH2 no IMAP e no SMTP com o access token da conta.
- Teste de conexão que abre o navegador e valida o retorno, como no fluxo atual.
- Sem senha no cofre para a conta OAuth; refresh token no cofre como hoje.
- Testes de unidade do fluxo e de regressão para as contas Gmail do catálogo.

### Critérios de aceite

- [ ] OAuth2 aparece só para Gmail manual; "Outro" e Outlook não mostram a opção.
- [ ] O teste de entrada e o de saída usam XOAUTH2 e não pedem senha.
- [ ] O refresh token continua no cofre, cifrado pela senha do Epistula quando ligada.
- [ ] O fluxo de autorização, callback, cancelamento e timeout é o mesmo do catálogo.
- [ ] A conta Gmail manual lê, envia e renova o token como a conta Gmail do catálogo.
- [ ] Nenhum token ou código aparece em log, banco, evento ou WebView.
- [ ] Modelo de ameaças, docs (pt/en/es/fr/it) e CHANGELOG atualizados.

### Base já implementada

A configuração manual, o cofre de senhas e o envio por SMTP com senha normal já estão no código (`src/` e `src-tauri/`).

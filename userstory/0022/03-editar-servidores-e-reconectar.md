## Fatia 03: Editar servidores e reconectar

**Parent:** `userstory/0022-configuracao-manual-de-servidores.md`

**Tipo:** AFK

### O que construir

Uma tela para ver e mudar a configuração de uma conta manual depois de criada, sem apagar as mensagens locais: servidor de entrada/saída, portas, segurança, método, usuário e senha. "Reconectar" de uma conta manual abre o mesmo formulário preenchido, em vez do fluxo OAuth.

Inclui:

- Configurações → Conta → Servidores, mostrando a configuração sem a senha.
- Edição com "Testar conexão" obrigatório antes de salvar.
- Salvar derruba as sessões IMAP da conta, atualiza o cofre e faz a próxima sincronização usar a configuração nova.
- Contas OAuth2 mostram os servidores do catálogo, somente leitura.
- Reconexão de conta `needs_reauth`/`signed_out` manual pelo formulário; OAuth continua no fluxo atual.

### Critérios de aceite

- [ ] A tela mostra host, porta, segurança, método e usuário de entrada e saída; nunca mostra a senha.
- [ ] Salvar sem testar é bloqueado; teste com erro não altera a configuração.
- [ ] Salvar com sucesso derruba as sessões antigas e a sincronização seguinte usa o servidor novo.
- [ ] Trocar a senha atualiza o cofre e mantém o resto da configuração.
- [ ] "Reconectar" de conta manual abre o formulário preenchido; de conta Gmail/Outlook continua no OAuth.
- [ ] Conta manual `signed_out` continua com as mensagens locais e volta a sincronizar depois de reconectar.
- [ ] Testes de backend e frontend cobrem edição, teste falho, troca de senha e reconexão.
- [ ] Docs (pt/en/es/fr/it) e CHANGELOG atualizados.

### Bloqueada por

- `userstory/0022/02-saida-smtp-senha-normal.md`

## Fatia 01: Entrada IMAP manual com senha normal

**Parent:** `userstory/0022-configuracao-manual-de-servidores.md`

**Tipo:** AFK

**Status:** implementada em 29 out 2026 (normal password; testes de rede com servidores falsos ainda pendentes).

### O que construir

O primeiro caminho completo de leitura: uma conta "Outro" configurada à mão, com servidor IMAP, usuário, senha e segurança da conexão, lendo a Caixa de entrada. O envio fica para a fatia 02; os campos de saída podem ser preenchidos e salvos, mas o app avisa que o envio ainda não está ligado para contas manuais.

Inclui:

- Migração nova que reconstrói `provider_config` com protocolo, configuração de entrada/saída, método de autenticação e usuário. Contas Gmail/Outlook existentes são copiadas sem perda.
- Modelo de configuração no Rust e leitura no `AccountHandle`, que deixa de exigir OAuth2.
- `state.credentials()` devolvendo token OAuth ou senha; o refresh automático continua só para OAuth.
- SASL PLAIN e LOGIN no IMAP, com fallback para o comando `LOGIN` quando o servidor não anunciar SASL.
- Senha da entrada no cofre, com partes, `Zeroizing` e `sealed1:` quando a senha do Epistula estiver ligada.
- Formulário manual com validação de host, porta, usuário e senha; "Testar conexão" autentica e confirma a Caixa de entrada sem baixar mensagens. Nesta fatia, só o teste de entrada é obrigatório; o teste de saída e o envio entram na fatia 02.
- Criação da conta com `provider = "custom"`, nome e cor padrão, e sincronização/leitura iguais às contas existentes.

### Critérios de aceite

- [ ] Servidor, porta e usuário são validados; porta fora de 1–65535, host com esquema/caminho/caractere de controle e campos vazios são recusados com mensagem amigável.
- [ ] Segurança aceita apenas SSL/TLS e STARTTLS; o certificado é validado.
- [ ] "Testar conexão" distingue DNS, TLS, autenticação e timeout, sem baixar mensagens.
- [ ] O teste usa PLAIN, depois LOGIN e, se não houver SASL, o comando `LOGIN`; um método recusado não vira outro silenciosamente.
- [ ] A conta criada sincroniza pastas, lista mensagens, abre corpos, marca flags e busca como uma conta IMAP existente; o envio manual fica explicitamente "Em breve" até a fatia 02.
- [ ] A senha nunca aparece no banco, em arquivo, em log, em evento Tauri ou na WebView; o teste de redaction continua passando.
- [ ] Com a senha do Epistula ligada, a senha da conta só é legível com o app desbloqueado; ao bloquear, as sessões e a senha em memória somem.
- [ ] Contas Gmail e Outlook existentes continuam abrindo e sincronizando depois da migração.
- [ ] `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `pnpm test` e `pnpm build` passam.
- [ ] CHANGELOG, modelo de ameaças e docs (pt/en/es/fr/it) descrevem a configuração manual de entrada.

### Bloqueada por

- `userstory/0022/00-decisoes-desenho-e-ameacas.md`

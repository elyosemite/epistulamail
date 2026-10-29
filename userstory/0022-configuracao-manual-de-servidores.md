## User Story 22: Configuração manual de servidores e métodos de autenticação

**Como** usuário que tem e-mail em um domínio próprio ou em um provedor fora do catálogo,
**quero** informar os servidores de entrada e de saída e escolher como o EpistulaMail se autentica,
**para que** eu leia e envie meus e-mails sem depender de OAuth nem de um provedor pré-configurado.

---

### 1. Contexto

Hoje o app é, na prática, OAuth2-only: o catálogo (`providers.rs`) só expõe Gmail e Outlook; `provider_config` (`migrations/0001_inicial.sql`) tem apenas `imap_*`, `smtp_*` e `auth_kind`; `session.rs` recusa qualquer conta que não seja Google/Microsoft; `imap_client.rs` e `send.rs` fixam XOAUTH2; e `secrets.rs` só guarda refresh token. O cartão "Outro" existe, mas está indisponível.

Esta história abre a configuração manual de uma conta **IMAP** com servidor de entrada e de saída independentes, no modelo do Thunderbird: host, porta, segurança da conexão, método de autenticação, usuário e senha. POP3 fica para a US23, porque exige um motor de sincronização próprio. O envio com CRAM-MD5, NTLM e GSSAPI fica para uma história futura (D4): o `lettre` só oferece PLAIN, LOGIN e XOAUTH2, e trocar o transporte SMTP é uma decisão separada.

A base de dados continua cifrada com SQLCipher. As senhas vão para o cofre do sistema, com o mesmo tratamento do refresh token (partes, `Zeroizing` e, com a senha do Epistula ligada, `sealed1:`). Nada de senha no banco, em arquivo, no log ou na WebView.

### 2. Critérios de aceite

**CA1. Cadastro manual**

- O cartão "Outro" fica disponível e abre um formulário com duas seções: **Entrada** e **Saída**.
- Entrada: protocolo (IMAP nesta história), servidor, porta, segurança da conexão, método de autenticação, usuário e senha.
- Saída: servidor, porta, segurança da conexão, método de autenticação, usuário e senha, com "Usar as mesmas credenciais da entrada" ligado por padrão.
- Portas padrão são sugeridas ao escolher protocolo/segurança (IMAP 993/143; SMTP 465/587), mas continuam editáveis.
- Servidor e usuário são validados: sem esquema, caminho, espaços nas pontas ou caracteres de controle; porta entre 1 e 65535; campos obrigatórios.
- "Testar conexão" testa entrada e saída separadamente e mostra o resultado de cada uma.
- Sem os dois testes aprovados, a conta não é salva; o usuário pode corrigir e testar de novo.

**CA2. Entrada e métodos de autenticação**

- **Senha normal:** IMAP com `AUTH=PLAIN`; se o servidor não anunciar PLAIN, `AUTH=LOGIN`; se não houver SASL, o comando `LOGIN` como último recurso.
- **Senha criptografada:** CRAM-MD5 (RFC 2195), quando o servidor anunciar `AUTH=CRAM-MD5`; sem suporte, erro amigável e sugestão de senha normal.
- **NTLM:** SASL NTLM, após a decisão D8; sem suporte no servidor, erro amigável.
- **Kerberos/GSSAPI:** SASL GSSAPI, após a decisão D9; sem suporte no servidor ou sem ticket, erro amigável.
- **OAuth2:** só para os provedores do catálogo (D3). Na prática, a conta Gmail manual usa IMAP/XOAUTH2; a Microsoft continua no Graph e não ganha configuração manual de servidor.
- O método escolhido é conferido contra as capacidades do servidor no teste de conexão; o app não faz downgrade silencioso para outro método.

**CA3. Saída e métodos de autenticação**

- **Senha normal:** SMTP com PLAIN/LOGIN sobre TLS ou STARTTLS.
- **OAuth2:** XOAUTH2 para a conta Gmail manual; a Microsoft continua no Graph.
- **Senha criptografada, NTLM e GSSAPI:** aparecem como "Em breve" no seletor de saída e não podem ser salvas nesta entrega (D4).
- "Usar as mesmas credenciais da entrada" copia usuário e senha; desligado, a saída tem usuário e senha próprios.
- O teste de saída faz conexão, EHLO e autenticação, sem enviar mensagem.

**CA4. Segurança da conexão**

- Opções: **SSL/TLS**, **STARTTLS** e **Detectar automaticamente**. "Sem segurança" não existe nesta versão (D2).
- "Detectar automaticamente" testa TLS implícito e, se falhar, STARTTLS no mesmo host/porta, sem enviar credenciais; se nenhum funcionar, a conta não é salva e a mensagem explica o motivo.
- A detecção grava a segurança resolvida (`tls` ou `starttls`), não "auto".
- O certificado é sempre validado; não há opção de aceitar certificado inválido.
- A detecção não busca configuração na internet (autoconfig/ISPDB/DNS); é só um probe de segurança no host informado (D6).

**CA5. Credenciais**

- Senhas de entrada e de saída ficam no cofre do sistema, em entradas por conta, separadas do refresh token.
- Com a senha do Epistula ligada, as senhas são cifradas por ela com o prefixo `sealed1:`; com o app bloqueado, não são acessíveis.
- As senhas são `Zeroizing`; nunca aparecem em `Debug`, log, banco, arquivo, evento Tauri ou estado React.
- Sair da conta apaga as senhas e o refresh token (se houver); "Apagar também os dados" apaga tudo.
- Falha de autenticação marca a conta como `needs_reauth` e mantém a configuração para o usuário corrigir; não apaga a senha sem pedir.

**CA6. Teste de conexão**

- O teste de entrada autentica e confirma que a Caixa de entrada existe (ou lista as pastas), sem baixar mensagens.
- O teste de saída autentica e encerra a sessão, sem enviar mensagem.
- Timeout de conexão e de operação com mensagens distintas; erros de DNS, TLS, autenticação e servidor indisponível têm mensagens amigáveis em português.
- O log registra só o tipo de erro e o tempo; nunca host, usuário, senha ou e-mail.

**CA7. Persistência e migração**

- Migração nova reconstrói `provider_config` com: protocolo (`imap`), configuração de entrada (`host`, `port`, `security`, `auth`, `username`) e de saída (`host`, `port`, `security`, `auth`, `username`), mais a indicação de credenciais compartilhadas.
- Contas Gmail e Outlook existentes são migradas sem perder servidores, segurança ou status.
- Conta manual usa `provider = "custom"`; a unicidade `(provider, email)` continua valendo.
- A configuração é lida do banco na criação do `AccountHandle`; nada de configuração fixa no código para contas manuais.
- O índice do cofre continua com id, provedor e e-mail; se o banco for recriado, a configuração manual precisa ser informada de novo (documentado).

**CA8. Sessão, leitura e envio**

- `AccountHandle` deixa de ser OAuth-only: carrega a configuração resolvida e o método de autenticação.
- `state.credentials()` devolve token OAuth ou senha, conforme a conta e a ponta; o refresh automático continua só para OAuth.
- IMAP usa o mecanismo escolhido; SMTP usa PLAIN/LOGIN/XOAUTH2 conforme a conta; Graph continua intocado para Outlook.
- Pastas, sincronização, leitura, busca, flags, mover, arquivar, excluir e regras funcionam como no Gmail para a conta IMAP manual.
- Envio usa a configuração de saída da conta e respeita anexos, Cco, "Desfazer envio" e os limites atuais.

**CA9. Edição e reconexão**

- Configurações → Conta → Servidores mostra a configuração atual sem a senha, para contas manuais.
- O usuário pode mudar host, porta, segurança, método, usuário e senha; "Testar conexão" é obrigatório antes de salvar.
- Salvar derruba as sessões IMAP da conta, atualiza o cofre e faz a próxima sincronização usar a configuração nova.
- "Reconectar" de uma conta manual abre o mesmo formulário, preenchido, em vez do fluxo OAuth.
- Contas OAuth2 mostram os servidores do catálogo, somente leitura.

**CA10. Segurança, privacidade e documentação**

- O modelo de ameaças cobre: senha em repouso e em memória, host informado pelo usuário, downgrade de STARTTLS na detecção, fraquezas de CRAM-MD5/NTLM, dependências de GSSAPI, autenticação recusada e vazamento em log.
- O inventário de criptografia e a tabela de ameaças são atualizados com as senhas manuais e com a ausência de "sem TLS".
- Docs em pt, en, es, fr e it: nova página "Conectar outro e-mail (IMAP)", atualização de privacidade e do que sai do computador; CHANGELOG na próxima versão.
- READMEs e páginas legais continuam coerentes com o que o app faz.
- Nenhum conteúdo de mensagem, endereço, host ou credencial vai para log.

**CA11. Testes e verificação**

- Testes de unidade: validação de host/porta/segurança/método, ordem da detecção, vetores SASL (PLAIN, LOGIN, CRAM-MD5, NTLM, GSSAPI, XOAUTH2), cofre com e sem senha do Epistula, migração preservando contas existentes.
- Testes de integração com servidores IMAP/SMTP falsos em loopback para PLAIN, LOGIN, CRAM-MD5, NTLM e falhas de autenticação; sem rede externa.
- Testes de frontend (`us22.test.ts`) para o formulário, estados de teste, "mesmas credenciais", validação e mensagens.
- `pnpm test`, `pnpm build`, `pnpm ds:check`, `cargo clippy --all-targets --locked -- -D warnings` e `cargo test --locked` verdes.

### 3. Interface

Rascunho aprovado em 29 out 2026 e apagado. O formulário é composição das peças existentes, sem peça nova no design system: `.field`, `.input`, `.input--password`, `.select`, `.section-message`, `.btn`, `.modal` e o padrão de validação (`padrao-validacao.html`). O rascunho mostrava formulário vazio, preenchido, testando, sucesso, erro de autenticação, erro de conexão e o aviso de envio avançado indisponível.

Telas: "Adicionar conta → Outro" e "Configurações → Conta → Servidores". O estado "Em breve" dos métodos de saída avançados segue o padrão de lozenge já usado no catálogo.

### 4. Decisões

- **D1. POP3 fica na US23.** Esta história entrega IMAP e SMTP; POP3 precisa de semântica própria (Inbox sintética, UIDL, exclusão, sem pastas/flags de servidor).
- **D2. Sem "sem segurança".** Só SSL/TLS, STARTTLS e detecção entre os dois. É um desvio consciente do Thunderbird, registrado no modelo de ameaças.
- **D3. OAuth2 manual só para os provedores do catálogo.** Reaproveita o cliente OAuth existente; na prática, só o Gmail manual ganha IMAP/XOAUTH2. O Outlook continua no Graph e "Outro" não oferece OAuth2. Endpoints genéricos ficam fora.
- **D4. Envio avançado adiado.** O `lettre` só faz PLAIN, LOGIN e XOAUTH2. CRAM-MD5, NTLM e GSSAPI no envio dependem de um transporte SMTP próprio ou de um fork, decisão ainda em aberto; nesta história o seletor de saída mostra os três como "Em breve" e não os salva.
- **D5. "Senha criptografada" é CRAM-MD5.** É desafio/resposta; a senha ainda precisa ficar no cofre para responder ao desafio. Não confundir com senha de app nem com criptografia de armazenamento.
- **D6. Detecção é só de segurança.** O app não busca autoconfig, ISPDB nem DNS; testa TLS e STARTTLS no host/porta informados.
- **D7. Registro local.** A história e as fatias ficam em `userstory/`, como as demais.
- **D8. NTLM em aberto.** Crate e cobertura por plataforma serão decididos na fatia 06 (HITL), antes de qualquer código.
- **D9. GSSAPI em aberto.** SSPI no Windows e GSSAPI do sistema no Linux têm custos de build e release diferentes; a fatia 07 começa por um spike HITL.
- **D10. Conta manual é `provider = "custom"`.** A unicidade `(provider, email)` continua; a configuração real vive em `provider_config`.

### 5. Fora do escopo

- POP3 (US23).
- Envio com CRAM-MD5, NTLM e GSSAPI (D4).
- OAuth2 genérico para domínios próprios (D3).
- Autoconfiguração de servidor por DNS/ISPDB/Thunderbird autoconfig.
- Conexão sem TLS (D2).
- Detecção de configuração de servidor além do probe de segurança.
- Contatos, calendário, OneDrive e outros serviços.

### 6. Fatias

As fatias estão em `userstory/0022/`, na ordem de dependência. Cada uma entrega UI, backend, migração/secrets, testes, docs e atualização do modelo de ameaças quando aplicável.

| # | Fatia | Tipo | Depende |
|---|---|---|---|
| 00 | Decisões, rascunho do DS e modelo de ameaças | HITL | — |
| 01 | Entrada IMAP manual com senha normal | AFK | 00 |
| 02 | Saída SMTP com senha normal | AFK | 01 |
| 03 | Editar servidores e reconectar | AFK | 02 |
| 04 | Detecção automática de segurança | AFK | 03 |
| 05 | Senha criptografada (CRAM-MD5) na entrada | AFK | 01, 02 |
| 06 | NTLM na entrada | HITL + AFK | 01, 02 |
| 07 | Kerberos/GSSAPI na entrada | HITL + AFK | 01, 02 |
| 08 | OAuth2 em conta Gmail manual | AFK | 01, 02 |

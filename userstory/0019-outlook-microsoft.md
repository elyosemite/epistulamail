## User Story 19: Outlook, Hotmail e Microsoft 365

> **Atualização (US20).** O Outlook passou a usar o Microsoft Graph. As permissões de IMAP/SMTP, o tenant com Exchange Online e o Manifesto desta história não são mais necessários; ver `0020-outlook-graph.md`.

**Como** usuário de Outlook.com, Hotmail ou Microsoft 365,
**quero** conectar a minha conta no EpistulaMail pelo login da Microsoft,
**para que** eu leia, busque, escreva e organize os meus e-mails no meu computador, com os mesmos cuidados de dados das contas Google.

---

### 1. Contexto

O catálogo de provedores (`providers.rs`) já tem o Outlook com IMAP e SMTP, e o `Provider::Microsoft` já tem as URLs de autorização e de token, os escopos e o PKCE em `provider.rs` e `oauth.rs`. O Outlook está com `available: false` para não aparecer na tela de conexão. O resto do caminho, IMAP e SMTP com XOAUTH2, refresh token e cofre do sistema, é o mesmo do Gmail. Como o código é compartilhado, o Linux ganha a funcionalidade junto com o Windows; o job Linux da release já recebe `EPISTULA_MS_CLIENT_ID`.

### 2. Critérios de aceite

**CA1. Login**

- O Outlook aparece na tela de adicionar conta e abre a página da Microsoft no navegador do sistema.
- O retorno acontece em `127.0.0.1` numa porta efêmera, com PKCE, `state` de uso único e timeout de 5 minutos.
- O app lê o e-mail das claims do `id_token` (`email` ou `preferred_username`) e cria a conta com o nome e a cor padrão.

**CA2. Leitura e envio**

- IMAP em `outlook.office365.com:993` com TLS e XOAUTH2; SMTP em `smtp.office365.com:587` com STARTTLS e XOAUTH2.
- Pastas, sincronização, busca, conversas, regras, download offline e envio funcionam como no Gmail.
- O refresh token é renovado antes de expirar e o refresh automático do token usa o escopo no endpoint da Microsoft.

**CA3. Dados e privacidade**

- O EpistulaMail pede só o necessário: `https://outlook.office.com/IMAP.AccessAsUser.All`, `https://outlook.office.com/SMTP.Send`, `offline_access`, `openid` e `email`. Não pede contatos, calendário, arquivos, OneDrive nem perfil além do endereço e do nome.
- A senha da conta nunca passa pelo app; o login acontece na página da Microsoft.
- O refresh token fica no cofre do sistema, com o mesmo tratamento do Gmail: nunca no banco em texto, nunca em arquivo, nunca no log; com a senha do Epistula ligada, também cifrado por ela.
- Nenhuma mensagem, assunto, endereço ou token passa por um servidor do EpistulaMail. Continua sem telemetria e sem relatório de erro.
- Sair da conta apaga o refresh token do cofre. Como a Microsoft não tem endpoint público de revogação para apps nativos, a tela e a documentação explicam como revogar o acesso na conta Microsoft.
- O modelo de ameaças, a política de privacidade, o site e os READMEs passam a falar de Google e Microsoft, com os escopos e os limites de cada uma.

**CA4. Linux e Windows**

- A mesma funcionalidade aparece nos dois sistemas, sem código específico de provedor por plataforma.
- No Linux, o login usa o navegador do sistema e o token fica no Secret Service; no Windows, no Gerenciador de Credenciais.

**CA5. Documentação e testes**

- Docs em pt, en, es, fr e it: conectar o Outlook, sair da conta e o que sai do computador.
- Testes do catálogo, clippy e testes de backend verdes nos dois sistemas; o CI Linux continua passando.

### 3. Interface

Nenhuma tela nova. O cartão do Outlook no `.provider-grid` já existe e passa a ficar disponível; o fluxo de espera, erro e reconexão é o mesmo do Gmail. A página de documentação "Conectar o Outlook" entra na seção Contas.

### 4. Decisões

- **D1. Habilitar o catálogo.** O Outlook vira `available: true`; o teste que hoje garante "só Gmail" passa a garantir "Gmail e Outlook".
- **D2. Escopos mínimos.** Só IMAP, SMTP, `offline_access`, `openid` e `email`; nada de contatos, calendário ou arquivos.
- **D3. Sem revogação automática na Microsoft.** Não há endpoint público para apps nativos; sair apaga o token local e a documentação indica a página de consentimentos da Microsoft.
- **D4. Redirect de loopback.** O app continua em `127.0.0.1` com porta efêmera; o registro do app no Entra ID precisa aceitar esse redirect. Se o portal recusar o IP, usar o Manifest, como já está no README.
- **D5. Foto do perfil.** O `photo.rs` é específico do Google; contas Microsoft usam a inicial e a cor até existir uma foto no `id_token` ou uma busca opcional.
- **D6. Dados.** A história de dados não muda: cofre do sistema, banco local cifrado, nenhum servidor nosso, nenhuma telemetria e logs sem conteúdo. A Microsoft entra no inventário e na política de privacidade.
- **D7. Conta pessoal sem tenant.** O diretório de uma conta pessoal da Microsoft não tem o service principal do Exchange Online (confirmado com `az ad sp show --id 00000002-0000-0ff1-ce00-000000000000`, que retorna não encontrado), então o Manifesto não resolve. Para Outlook.com pessoal, o app registration precisa estar num tenant de trabalho/escola com Exchange Online (a conta pessoal pode ser a usuária) ou o backend precisa ser o Microsoft Graph. Esta história cobre o IMAP/SMTP com o primeiro caminho; o Graph fica para uma próxima história.

### 5. Fora do escopo

- Yahoo, iCloud, Zoho, GMX, AOL e IMAP genérico: continuam indisponíveis.
- Calendário, contatos, OneDrive e Microsoft Graph.
- Consentimento de administrador automático em Microsoft 365: o administrador do tenant continua responsável por liberar IMAP/SMTP.

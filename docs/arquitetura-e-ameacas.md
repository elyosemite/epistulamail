# Epistula: arquitetura e modelo de ameaças

Situação em 23 out 2026, após a US2. Produto: cliente desktop que conecta uma conta Gmail via OAuth e lê a caixa por IMAP (decisão D1, opção A, provisória). Não há servidor do Epistula.

## Componentes e limites de confiança

```
┌──────────────────────── Computador do usuário ────────────────────────┐
│                                                                       │
│  WebView (React)  ──IPC (comandos Tauri)──▶  Processo Rust            │
│   · só exibe dados                            · OAuth (PKCE, loopback)│
│   · nunca vê tokens                           · IMAP XOAUTH2 + TLS    │
│   · HTML de e-mail num iframe                 · parse MIME + limpeza  │
│     sandbox sem scripts                       · sessão IMAP em memória│
│                                                      │                │
│                                         Keyring do SO (refresh token) │
└──────────────────────────────────────────────┬────────────────────────┘
                                               │ TLS
                         ┌─────────────────────┴──────────────────────┐
                         │ Google: accounts.google.com, oauth2.googleapis.com,
                         │ imap.gmail.com:993                          │
                         └─────────────────────────────────────────────┘
```

Limites de confiança, do mais confiável ao menos:

1. **Processo Rust.** Guarda credenciais, decide o que sai para a WebView e o que vai para o log.
2. **WebView do app.** Código do Epistula. Recebe só dados de exibição; a CSP proíbe scripts externos e `eval`.
3. **Conteúdo de e-mail de terceiros.** Não confiável. Cabeçalhos são texto; HTML passa pelo `ammonia` no Rust e é exibido num `<iframe sandbox>` sem `allow-scripts`, com CSP própria.
4. **Google.** Provedor externo. Tem acesso a todo o conteúdo da conta, por definição.

## Matriz de dados

| Dado | Onde fica | Por quanto tempo | Quem acessa |
|---|---|---|---|
| Refresh token OAuth | Keyring do SO (`epistula/secret-N`) | Até desconectar, `invalid_grant` ou 7 dias (app em modo Testing) | Processo Rust; usuário do SO |
| E-mail da conta, provedor | Keyring (`epistula/account`) e memória | Até desconectar | Processo Rust; WebView (exibição) |
| Access token OAuth | Memória do processo Rust | ~1 h ou até fechar o app | Processo Rust |
| Sessão IMAP autenticada | Memória do processo Rust | Até erro, troca de conta ou fechar o app | Processo Rust |
| Cabeçalhos e corpo de mensagens | Memória (Rust e WebView) | Enquanto exibidos; nada vai para disco | Processo Rust; WebView |
| `client_id` e `client_secret` do Google | `.env` local (dev) ou binário (build) | Permanente | Qualquer pessoa com o binário (o Google não trata o secret de app desktop como confidencial) |
| Logs | stdout do processo | Sessão do terminal | Usuário do SO |

Os logs contêm só: comando, papel da pasta, contagens, tamanhos, duração e códigos de erro. Um teste (`message::tests::logs_never_contain_message_content`) captura os logs da suíte inteira e falha se aparecer assunto, endereço, corpo, nome de anexo ou domínio de rastreador.

## Ameaças

| # | Ameaça | Situação | Controles e riscos residuais |
|---|---|---|---|
| 1 | Atacante externo sem autenticação | Mitigada | Não há porta aberta além do loopback efêmero do OAuth: 127.0.0.1, aceita uma requisição, valida `state` de uso único e expira em 5 min. |
| 2 | Usuário autenticado acessando dados de outra conta | Fora do escopo | Uma conta por instalação; o isolamento entre contas é responsabilidade do Google. |
| 3 | Roubo de banco ou backup | Mitigada | Não há banco. O refresh token fica no keyring do SO; o conteúdo não é persistido. Backups do perfil do SO incluem o keyring cifrado pelo SO. |
| 4 | Conta administrativa comprometida | Fora do escopo | Não há administração do Epistula. Um admin de Workspace pode revogar ou auditar o acesso pelo Google. |
| 5 | CI/CD ou cadeia de dependências | Parcialmente mitigada | `Cargo.lock` e `pnpm-lock.yaml` versionados; `cargo deny` (licenças, fontes, advisories) e `cargo audit` sem vulnerabilidades. Ainda não há CI nem auditoria das dependências npm, e os releases não são assinados. |
| 6 | Operador curioso ou malicioso | Fora do escopo | Não há operador do Epistula com acesso a dados. O Google, como provedor, tem acesso total (ver 8). |
| 7 | Dispositivo do usuário comprometido | Não mitigada | Malware com a sessão do usuário lê o keyring, a memória e a tela. Nenhum cliente de e-mail resolve isso. |
| 8 | Provedor externo sem E2EE | Aceita | **O Google lê todos os e-mails da conta.** O Epistula não muda isso; TLS protege só o trânsito. E2EE depende da decisão D3 (OpenPGP no cliente). |
| 9 | Abuso: phishing, rastreamento, conteúdo malicioso | Parcialmente mitigada | HTML limpo por allowlist no Rust (corpus de ataques + 768 casos de proptest); iframe sem scripts; imagens remotas bloqueadas até o usuário liberar por mensagem; links só `http`/`https`/`mailto`, abertos no navegador do sistema. Não há detecção de phishing nem verificação de SPF/DKIM na UI. |
| 10 | Perda, roubo ou revogação de chaves | Mitigada para OAuth | Revogar em `myaccount.google.com/permissions` invalida o refresh token; o app detecta `invalid_grant`, apaga o keyring e pede nova conexão. Não há chaves E2EE ainda. |
| 11 | Logs, traces, dumps de memória, temporários | Parcialmente mitigada | Logs sem conteúdo, com teste. Nada é escrito em disco. Dumps de memória do processo conteriam tokens e mensagens abertas; os segredos não são zerados ao sair da memória. |
| 12 | Infraestrutura ou ordem legal | Fora do escopo do Epistula | Não há infraestrutura do Epistula. Pedidos legais alcançam os dados no Google. |

## Premissas

- O keyring do SO protege os itens da sessão do usuário contra outros usuários locais.
- A WebView do sistema (WebView2 no Windows) aplica corretamente `sandbox`, `srcdoc` e CSP.
- O `ammonia` (allowlist sobre o parser html5ever) é a barreira principal contra HTML ativo; o iframe sem scripts é a segunda.
- Testes contra o Gmail real são manuais: não há servidor IMAP com XOAUTH2 em CI.

## Riscos residuais priorizados

1. Releases sem assinatura nem atualização verificada (ameaça 5): antes de distribuir.
2. Verificação do app OAuth pelo Google, com avaliação de segurança, para sair do modo Testing.
3. CI com `cargo clippy`, testes, `cargo deny` e `cargo audit`, mais auditoria das dependências npm.
4. Indicadores de autenticidade do remetente (SPF/DKIM/DMARC dos cabeçalhos `Authentication-Results`).

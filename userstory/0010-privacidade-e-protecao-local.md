## User Story 10: Privacidade e proteção dos dados guardados no computador

**Como** usuário do Epistula,
**quero** que os e-mails guardados no meu computador só possam ser lidos por mim, mesmo por quem tiver acesso ao meu Windows ou a uma cópia do disco, e que nada do que eu leio vaze para fora do banco cifrado,
**para que** a minha cópia local seja de fato minha, hoje e quando existirem computadores quânticos.

---

### 1. Contexto

Pedido em 25 out 2026 como "criptografia pós-quântica" e refeito em 26 out 2026. Hoje:

- **Banco local:** SQLCipher 4, com AES-256 por página e HMAC-SHA512. A chave tem 32 bytes do gerador do sistema e fica só no Cofre de Credenciais do Windows (crate `keyring`), protegida pelo DPAPI (US3, D2). Já usa `secure_delete = ON` e `temp_store = MEMORY` (`db.rs`).
- **Rede:** IMAP, SMTP e login usam TLS pelo Windows (`native-tls`/SChannel), com troca de chaves por curva elíptica. Os provedores ainda não oferecem troca de chaves pós-quântica no IMAP e no SMTP.

**O que já resiste a computadores quânticos:** a cifra do banco. Um computador quântico quebra a criptografia de chave pública (RSA, curvas elípticas). Contra a simétrica, o algoritmo de Grover só reduz a força pela metade: AES-256 fica com cerca de 128 bits, fora de alcance. Não existe cifra simétrica "pós-quântica" melhor para pôr no lugar.

**Onde estão os riscos de verdade:**

1. **Quem consegue a chave.** O DPAPI entrega a chave a quem está na sessão do Windows do usuário. Também a entrega a quem copia o disco e tem a senha do Windows, que, num disco sem BitLocker, dá para tentar adivinhar offline. E a entrega ao administrador de um domínio, que tem a chave de backup do DPAPI (protegida por RSA, que não resiste a computadores quânticos).
2. **O que vaza para fora do banco cifrado:**
   - o texto das notificações, que o Windows guarda num banco próprio sem cifra;
   - o cache do WebView2, quando o usuário libera as imagens externas;
   - os despejos de memória de uma falha;
   - o arquivo de paginação e o de hibernação.
3. **O que fica guardado sem precisar:** e-mails fora do período escolhido (D4 da US9) e e-mails de contas que o usuário deixou.
4. **Rastreamento na leitura:** links que levam a um endereço diferente do texto mostrado e parâmetros de rastreamento nos links.
5. **Trânsito:** quem gravar hoje o tráfego pode decifrá-lo no futuro ("colher agora, decifrar depois"). A troca de chaves híbrida nas conexões saiu desta story e foi para a US11 (`0011-tls-hibrido-pos-quantico.md`), por causa dos riscos com outros provedores e outros sistemas.

### 2. Regras (obrigatórias)

- Nenhuma criptografia própria: só bibliotecas mantidas e auditadas, com algoritmos padronizados (Argon2id RFC 9106, AES-256-GCM, FIPS 203 ML-KEM, RFC 9180 HPKE).
- Nunca enfraquecer a proteção atual, nem para passar teste, nem como etapa intermediária.
- Logs nunca levam chave, senha, segredo, conteúdo, endereço de link ou parte deles.
- Toda peça nova de interface passa primeiro por um rascunho em `design-system/rascunho-us10.html`: a tela de desbloqueio, as opções de Privacidade e o aviso de link. Depois da aprovação, o rascunho vai para as páginas do DS.
- Nenhum texto da interface cita o provedor de e-mail.

### 3. Critérios de aceite

**CA1. Modelo de ameaça atualizado**

- `docs/arquitetura-e-ameacas.md` ganha uma seção de criptografia: cada uso no app (onde, para quê, algoritmo, biblioteca, simétrica ou de chave pública, situação diante de computadores quânticos, o que fazer).
- As ameaças novas entram na tabela: cópia do disco com a senha do Windows, backup do DPAPI em domínio, notificações, cache do WebView2, despejos de memória, paginação e hibernação.
- Fica escrito o que o app não protege: malware rodando na sessão com o app desbloqueado.

**CA2. Senha do Epistula (D1)**

- Configurações → Privacidade: "Pedir uma senha ao abrir o Epistula", desligada por padrão.
- Ao ligar, a tela avisa antes de confirmar: esquecer a senha significa perder as mensagens guardadas no computador; elas continuam no servidor e podem ser baixadas de novo.
- A chave do banco passa a ser guardada envolvida (AES-256-GCM) por uma chave derivada da senha com Argon2id (perfil da RFC 9106 para desktop: cerca de 64 MB de memória e meio segundo no computador do usuário). A senha não é gravada em lugar nenhum.
- Ao abrir o app com a senha ligada, aparece a tela de desbloqueio antes de qualquer conteúdo. Nada sincroniza nem notifica antes do desbloqueio.
- Senha errada: mensagem genérica e espera crescente entre tentativas, sem apagar nada.
- Trocar a senha e desligar a senha só envolvem a chave de novo, sem recriar o banco.
- Com a senha ligada, um arquivo do banco copiado junto com o Cofre de Credenciais não abre sem ela, nem pela chave de backup do domínio.

**CA3. Bloqueio do app (D2)**

- Com a senha ligada, o app bloqueia depois de um tempo sem uso (padrão 15 minutos; opções 5, 15, 30, 60 minutos ou nunca) e quando o Windows bloqueia a tela.
- "Bloquear agora" no menu da conta e na bandeja.
- Bloquear fecha o banco, apaga da memória a chave e o conteúdo aberto e mostra a tela de desbloqueio.
- Enquanto o app está bloqueado, a sincronização e o download em segundo plano param, e as notificações mostram só a conta (D3).

**CA4. Chave do banco: configurações fixas e troca**

- As configurações do SQLCipher ficam fixas no código (`cipher_compatibility = 4`, `cipher_hmac_algorithm`, `cipher_kdf_algorithm`, `cipher_page_size`), em vez de depender do padrão da biblioteca. Um teste falha se mudarem.
- `cipher_memory_security`: testado ligado em 26 out 2026, estoura a pilha dentro do SQLCipher no Windows. Fica desligado de forma explícita (e testado); as chaves do app são zeradas pelo `zeroize`.
- O banco registra a versão do esquema de cifra, para uma troca futura saber de onde parte.
- Troca da chave (`PRAGMA rekey`) sem perder dados: a chave nova só vai para o cofre depois de o banco abrir com ela. Uma falha no meio deixa o banco abrindo com a chave antiga.
- Configurações → Privacidade: "Trocar a chave de proteção dos dados", com confirmação (D4).
- A chave também é trocada sozinha depois de apagar os dados de uma conta (CA7). Assim, uma cópia antiga do banco não abre com a chave atual.

**CA5. Segredos na memória**

- Chave do banco, chave derivada da senha, senha digitada e tokens OAuth ficam em tipos que se zeram ao sair de uso (crate `zeroize`) e não aparecem em `Debug`.
- O texto da chave em hexadecimal montado para o `PRAGMA key` também é zerado.
- O app sai dos relatórios de erro do Windows (`WerAddExcludedApplication`), para uma falha não gravar em disco um despejo de memória com e-mails e chaves.

**CA6. Vazamentos fora do banco**

- **Notificações (D5):** o padrão de "O que aparece na notificação" passa a ser "Só a conta". A descrição da opção avisa que o Windows guarda o texto das notificações sem cifra.
- **Cache do WebView2:** o conteúdo dos e-mails nunca é gravado no cache de disco. O cache é desligado ou limpo ao fechar o app e ao bloquear (D6). Um teste confere que, depois de abrir um e-mail com imagens liberadas, nada do e-mail fica na pasta de dados do WebView2.
- **Disco sem BitLocker:** o app confere se o disco onde fica o banco está cifrado. Se não estiver, Configurações → Privacidade recomenda ligar o BitLocker, que protege a paginação e a hibernação. Só uma recomendação, sem bloquear nada.

**CA7. Guardar menos (D7)**

- Configurações → E-mail → Ler sem conexão: "Apagar do computador os e-mails fora do período". Apaga só os corpos e anexos salvos; os cabeçalhos ficam e o e-mail baixa de novo ao abrir.
- Ao sair de uma conta, o diálogo oferece "Apagar também os e-mails desta conta guardados neste computador". Ao confirmar, apaga mensagens, pastas, rascunhos e avisos da conta, roda `VACUUM` e troca a chave do banco (CA4).
- Configurações → Privacidade mostra quanto espaço os dados guardados ocupam, por conta.

**CA8. Privacidade na leitura**

- **Link enganoso:** quando o texto de um link parece um endereço e o destino é de outro domínio, o clique mostra antes os dois endereços, e o usuário escolhe abrir ou não.
- **Rastreamento nos links (D8):** ao abrir um link, o app tira os parâmetros de rastreamento conhecidos (`utm_*`, `fbclid`, `gclid`, `mc_eid` e parecidos), numa lista fixa e testada.
- **Confirmação de leitura:** o app continua sem mandar confirmação de leitura (`Disposition-Notification-To`). Um teste garante isso.
- O bloqueio das imagens externas (US2) continua como está.

**CA9. Conexões com troca de chaves híbrida**

- Saiu desta story em 26 out 2026 e foi para a US11, com os riscos levantados.

**CA10. Regra para recursos futuros**

- Fica escrito no modelo de ameaça: todo recurso que precisar de chave pública (backup cifrado, sincronização entre dispositivos, e-mail cifrado entre pessoas) usa o modelo híbrido: X25519 + ML-KEM-768 via HPKE, ou X-Wing quando estiver padronizado. Assinaturas, quando forem necessárias, consideram ML-DSA (FIPS 204) em modo híbrido.
- Nenhum recurso atual precisa disso; é só a decisão escrita.

### 4. Implementações

Cada item é uma entrega separada, na ordem sugerida.

1. **Rascunho de interface** (`design-system/rascunho-us10.html`): tela de desbloqueio, grupo Privacidade nas Configurações (senha, bloqueio, troca de chave, espaço por conta, aviso do BitLocker), diálogo de ativar a senha com o aviso de perda, diálogo de sair com "apagar também", aviso de link enganoso, "Bloquear agora" no menu da conta e na bandeja. Espera aprovação.
2. **Modelo de ameaça** (CA1, CA10): seção de criptografia e ameaças novas em `docs/arquitetura-e-ameacas.md`.
3. **Segredos na memória** (CA5): `zeroize` e `secrecy` em `DbKey`, nos tokens e no `key_pragma` (`db.rs`, `secrets.rs`, `oauth.rs`, `session.rs`); `WerAddExcludedApplication` na abertura (`lib.rs`).
4. **Configurações fixas do SQLCipher** (CA4): pragmas de cifra explícitos e `cipher_memory_security` em `db.rs`, versão do esquema de cifra em `app_setting` (migração nova), teste que falha se mudarem.
5. **Troca da chave** (CA4): `Db::rekey` com a ordem segura; comando `rotate_db_key`; testes de troca, falha no meio e chave errada.
6. **Formato do segredo guardado com versão** (CA2): em `secrets.rs`, o cofre passa a guardar "chave crua" (v1, a de hoje) ou "chave envolvida pela senha" (v2: sal, parâmetros do Argon2id, nonce, texto cifrado). Lê os dois formatos.
7. **Senha do Epistula** (CA2): módulo `lock.rs` com Argon2id (crate `argon2`) e AES-256-GCM (crate `aes-gcm`); comandos `set_app_password`, `change_app_password`, `remove_app_password`, `unlock { password }`; espera crescente entre tentativas; a abertura (`lib.rs`, `session.rs`) espera o desbloqueio antes de abrir o banco, sincronizar e notificar.
8. **Tela de desbloqueio e Privacidade** (CA2, CA3, CA4): componentes aprovados no item 1, em `src/features/settings/PrivacySettings.tsx` e na tela de desbloqueio.
9. **Bloqueio** (CA3): contador de inatividade no frontend, evento de bloqueio da sessão do Windows (`WTSRegisterSessionNotification`), comando `lock_app` que para a sincronização e o download, fecha o banco, apaga a chave e avisa a interface; "Bloquear agora" no menu da conta e na bandeja (`tray.rs`).
10. **Notificações** (CA6): padrão "Só a conta" em `store.rs` (só para quem ainda não escolheu), nova descrição em `NotificationSettings.tsx`, notificações só da conta enquanto bloqueado (`notify.rs`).
11. **Cache do WebView2** (CA6): desligar o cache de disco (argumentos do navegador na janela) ou limpar os dados de navegação ao fechar e ao bloquear; teste que procura o conteúdo de um e-mail na pasta de dados.
12. **Aviso do BitLocker** (CA6): conferir se o volume do banco está cifrado (consulta ao Windows, sem pedir permissão de administrador) e mostrar a recomendação em Privacidade.
13. **Apagar fora do período** (CA7): `store::purge_bodies_before`, comando e botão em `OfflineSettings.tsx`.
14. **Apagar ao sair da conta** (CA7): opção no diálogo de sair; `store::delete_account_data`, `VACUUM` e troca da chave (item 5); espaço por conta em Privacidade.
15. **Links** (CA8): em `sanitize.rs`, marcar os links cujo texto parece endereço de outro domínio; no clique, o aviso aprovado; em `commands.rs` (abrir link), tirar os parâmetros de rastreamento; testes com a lista fixa e com links legítimos que não podem quebrar.
16. **Confirmação de leitura** (CA8): teste que garante que nenhum caminho envia confirmação de leitura.
17. **READMEs e story:** atualizar o que existe e registrar a implementação nesta story.

### 5. Decisões

- **D1 (aprovada em 26 out 2026: oferecer, desligada por padrão):** senha do Epistula ao abrir o app.
- **D2 (aprovada em 26 out 2026: 15 minutos e ao bloquear o Windows):** quando o app bloqueia sozinho.
- **D3 (aprovada em 26 out 2026: só a conta):** o que as notificações mostram com o app bloqueado. A alternativa é não notificar nada.
- **D4 (aprovada em 26 out 2026: só manual e depois de apagar uma conta):** troca da chave do banco. A alternativa é trocar sozinho uma vez por ano.
- **D5 (aprovada em 26 out 2026: "Só a conta" para quem ainda não escolheu):** novo padrão das notificações. Quem já escolheu "Remetente e assunto" continua com a escolha.
- **D6 (aprovada em 26 out 2026: desligar o cache de disco):** WebView2 sem cache de disco ou com cache limpo ao fechar e ao bloquear. Sem cache, as imagens liberadas baixam de novo a cada abertura.
- **D7 (aprovada em 26 out 2026: oferecer, sem apagar sozinho):** apagar do computador os e-mails fora do período e os de contas que o usuário deixou.
- **D8 (aprovada em 26 out 2026: sim):** tirar os parâmetros de rastreamento dos links sem perguntar.
- **D9:** foi para a US11.
- **D10 (decidida na story, 26 out 2026):** não cifrar o banco de novo com outro algoritmo. O AES-256 já resiste a computadores quânticos; a troca teria custo e risco sem ganho.

### 6. Andamento (26 out 2026)

- **Item 1:** rascunho aprovado em 26 out 2026, incorporado ao DS e apagado: L53 tela de desbloqueio (carregamento.html), L54 campo de senha (campos.html), L55 conferência de link, L56 espaço por conta e L57 confirmação em modal (modais.html), "Bloquear o Epistula" (header.html, toasts.html); ícones `lock`, `shield`, `key`, `eye-off`.
- **Item 2:** feito. `docs/arquitetura-e-ameacas.md` tem a seção Criptografia (inventário e regra para recursos futuros), as ameaças 15 a 17 e as ameaças 3 e 11 atualizadas.
- **Item 3:** feito. `DbKey` virou um tipo com `Zeroizing` e `Debug` sem a chave; o `PRAGMA key` é montado num texto zerado; refresh token e access token em `SecretString` (`secrets.rs`, `oauth.rs`, `session.rs`). As cópias dentro da `lettre`, do `keyring` e do SChannel ficam fora do alcance. O app sai dos relatórios de erro do Windows ao abrir (`lib.rs`).
- **Item 4:** feito. `CIPHER_PRAGMAS` em `db.rs`, migração `0006_cifra.sql` com `cipher_scheme = 1`, banco com esquema mais novo não abre; testes `cipher_settings_are_pinned` e `database_created_with_library_defaults_still_opens`.
- **Item 5:** feito no backend. `keys.rs` (`open_database` e `rotate`, com o cofre por trás de `KeyVault`) e o comando `rotate_db_key`; a chave nova fica em `db-key-next` até o banco abrir com ela. Testes de troca, de interrupção antes e depois do rekey e de falha ao gravar cada chave. Falta o botão (item 8).
- **Item 6:** feito. `StoredKey` lê v1 (hexadecimal) e v2 (JSON com versão, custo do Argon2id, sal, nonce e texto cifrado).
- **Item 7:** feito. `lock.rs` (Argon2id 64 MiB e 3 passadas, AES-256-GCM com dados associados por uso, custo lido do cofre só dentro de limites, espera de 2, 4, 8… até 300 s), `keys.rs` com `Protection` (chave crua ou envolvida), `privacy.rs` (desbloquear, bloquear, ligar, trocar e desligar a senha, "esqueci a senha"). **Achado de segurança:** só envolver a chave do banco não bastava, porque o refresh token continuava no cofre só com o DPAPI e daria acesso ao servidor a quem tem o Windows; com a senha ligada, os tokens também vão cifrados por ela (`sealed1:`), e "esqueci a senha" os apaga (as contas voltam pedindo para reconectar). O `Db` fica sem conexão enquanto bloqueado (`Locked`); a inicialização espera o primeiro desbloqueio.
- **Item 8:** feito. `LockGate` e `UnlockScreen` (L53, com espera e "Esqueci a senha"), `PasswordField` (L54), `ConfirmCheck` (L57), Configurações → Privacidade e os modais de senha, chave e dados.
- **Item 9:** feito. Tempo sem uso no frontend (eventos de teclado e mouse, padrão 15 min), Windows bloqueado (conferido a cada 3 s pela área de trabalho de entrada), Ctrl+Shift+L, "Bloquear o Epistula" no menu da conta e na bandeja. Bloquear encerra as conexões, descarta as sessões e os tokens da memória, fecha o banco e limpa os dados de navegação da janela. **Diferença do CA3/D3:** com o banco fechado não há sincronização, então não chegam notificações de e-mail enquanto o app está bloqueado (nem só com a conta).
- **Item 10:** feito. Padrão "Só a conta" (para quem nunca salvou as notificações), opção primeiro na lista e o aviso de que o Windows guarda o texto sem cifra.
- **Item 11:** feito em parte. `--disk-cache-size=1` nos argumentos do WebView2 e limpeza dos dados de navegação ao bloquear. Falta conferir no app real que nada do e-mail fica na pasta do WebView2 (não dá para testar no CI).
- **Item 12:** feito. `System.Volume.BitLockerProtection` do volume do banco, lido pelo PowerShell sem administrador; aviso só quando desligado ou suspenso.
- **Item 13:** feito. "E-mails fora do período" com o tamanho e "Apagar", que tira os corpos (os cabeçalhos ficam), compacta e troca a chave.
- **Item 14:** feito. "Apagar também os e-mails desta conta" ao sair (o botão vira "Sair e apagar") e "Apagar" em Privacidade só nas contas de que o usuário saiu: apaga a conta e tudo que pende dela, tira do índice do cofre, compacta e troca a chave. Em contas em uso o "Apagar" não existe, porque o download em segundo plano traria o conteúdo de volta.
- **Testes:** 138 no Rust e 67 no frontend passam; clippy e typecheck sem avisos.
- **Próximos:** itens 15 (links) e 16 (confirmação de leitura); teste manual do bloqueio e do WebView2 no app real.

### 7. Riscos técnicos

- **Senha esquecida:** perda das mensagens guardadas no computador. Por isso o aviso antes de ativar e a senha desligada por padrão.
- **Troca de chave interrompida:** a ordem das etapas e os testes do item 5 existem para o banco nunca ficar sem uma chave válida.
- **Bloqueio com tarefas em andamento:** sincronização, download e envio precisam parar sem corromper o banco nem perder um e-mail na fila de envio.
- **Argon2id em computador fraco:** os parâmetros são medidos e guardados junto da chave envolvida, para poderem mudar depois sem perder o acesso.
- **WebView2:** o controle do cache depende das opções que o Tauri expõe. Pode exigir limpar os dados de navegação pela API do WebView2.
- **Links:** tirar parâmetros pode quebrar links de confirmação que dependem deles. A lista é fixa, curta e testada.

### 8. Fora do escopo

- E-mail cifrado entre pessoas (S/MIME, OpenPGP) e um serviço próprio de criptografia de ponta a ponta.
- Backup cifrado e sincronização entre dispositivos (quando vierem, seguem o CA10).
- Proteção contra malware rodando na sessão do usuário com o app desbloqueado.
- Desbloqueio com Windows Hello ou TPM. Pode vir depois como atalho, mas nunca como a única proteção, porque essas chaves costumam ser RSA ou de curva elíptica.

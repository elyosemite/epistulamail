## User Story 10: Proteção dos dados contra computadores quânticos

**Como** usuário do Epistula,
**quero** que os e-mails guardados no meu computador e o que trafega entre o app e o servidor continuem protegidos quando existirem computadores quânticos capazes de quebrar a criptografia de hoje,
**para que** nada do que eu leio ou envio hoje possa ser lido por alguém no futuro.

---

### 1. Contexto

Pedido em 25 out 2026. Hoje:

- **Banco local:** SQLCipher 4 (AES-256 por página com HMAC-SHA512). A chave tem 32 bytes do gerador do sistema e fica só no Cofre de Credenciais do Windows (crate `keyring`), protegida pelo DPAPI, que é simétrico e ligado ao login do Windows (US3, D2).
- **Rede:** IMAP, SMTP e login usam TLS pelo Windows (`native-tls`/SChannel), com troca de chaves por curva elíptica.

Um computador quântico ameaça a criptografia de **chave pública** (RSA, curvas elípticas, a troca de chaves do TLS). A **simétrica** perde só metade da força pelo algoritmo de Grover: AES-256 fica com cerca de 128 bits de segurança efetiva, fora de alcance. Por isso o arquivo do banco, em si, já é resistente. Os riscos estão em outros pontos:

1. **"Colher agora, decifrar depois":** quem gravar hoje o tráfego TLS entre o app e o servidor poderá decifrá-lo quando quebrar a troca de chaves por curva elíptica.
2. **Backup do DPAPI em domínio:** em computadores de empresa ligados a um domínio, o Windows guarda uma cópia das chaves do DPAPI protegida por RSA no controlador de domínio.
3. **Agilidade criptográfica:** o banco não registra qual esquema de cifra usa, e não há como trocar a chave sem recriar tudo.
4. **Recursos futuros com chave pública** (backup cifrado, sincronização entre dispositivos, e-mail cifrado entre pessoas) precisam nascer pós-quânticos.

### 2. Regras (obrigatórias)

- Nenhuma criptografia própria: só bibliotecas mantidas e auditadas, com algoritmos padronizados (FIPS 203 ML-KEM, RFC 9180 HPKE, Argon2id RFC 9106).
- Nunca enfraquecer a proteção atual para passar teste nem como etapa intermediária.
- Logs nunca levam chave, senha, segredo ou parte deles.
- Toda peça nova de interface (a senha do Epistula) passa primeiro por um rascunho em `design-system/rascunho-us10.html` para aprovação.

### 3. Critérios de aceite

**CA1. Inventário e modelo de ameaça**

- Documento `docs/criptografia.md` com cada uso de criptografia do app: onde, para quê, algoritmo, biblioteca, se é simétrica ou de chave pública, situação diante de computadores quânticos e o que fazer.
- O modelo de ameaça diz o que o app protege (arquivo copiado do disco, tráfego gravado, computador perdido) e o que não protege (malware rodando na sessão do usuário com o app aberto).

**CA2. Banco com versão de cifra e troca de chave**

- As configurações do SQLCipher ficam fixas no código (`cipher_compatibility`, algoritmo de HMAC, tamanho de página), em vez de depender do padrão da biblioteca, e um teste falha se mudarem.
- O banco registra a versão do esquema de cifra (`app_setting` ou cabeçalho próprio), para uma migração futura saber de onde parte.
- Trocar a chave do banco (`PRAGMA rekey`) sem perder dados: a chave nova vai para o Cofre de Credenciais só depois de o banco abrir com ela; uma falha no meio deixa o banco abrindo com a chave antiga.
- Configurações → Conta (ou Privacidade): "Trocar a chave de proteção dos dados", com confirmação.

**CA3. Senha do Epistula (opcional, D1)**

- Uma opção para pedir uma senha ao abrir o app. A chave do banco passa a ser protegida por uma chave derivada dessa senha com Argon2id (parâmetros da RFC 9106, perfil para desktop), além do Cofre de Credenciais.
- Esquecer a senha significa perder o acesso às mensagens salvas (elas continuam no servidor). A tela diz isso antes de ativar.
- Ligar e desligar a senha troca a proteção da chave sem recriar o banco.
- Com a senha ligada, um arquivo do banco copiado junto com o Cofre de Credenciais não abre sem ela, nem pelo backup RSA do DPAPI em domínio.

**CA4. TLS híbrido pós-quântico nas conexões (D2)**

- IMAP, SMTP e as chamadas de login e foto usam rustls com o provedor aws-lc-rs, que oferece a troca de chaves híbrida X25519 + ML-KEM-768 e cai para a clássica quando o servidor não aceita.
- As raízes de confiança continuam vindo do Windows (ex.: `rustls-platform-verifier`), para respeitar certificados corporativos.
- O log de diagnóstico registra o grupo de troca de chaves negociado (ex.: `X25519MLKEM768`), sem nada da sessão.
- Antes de implementar: verificar se os servidores IMAP e SMTP do provedor aceitam o híbrido. Se não aceitarem, a troca continua valendo (fica pronta para quando aceitarem) e a story registra o resultado.

**CA5. Regra para recursos futuros**

- Fica registrado no `docs/criptografia.md`: todo recurso que precisar de chave pública usa o modelo híbrido (X25519 + ML-KEM-768 via HPKE, ou X-Wing quando estiver padronizado), com biblioteca auditada. Assinaturas, quando forem necessárias, consideram ML-DSA (FIPS 204) em modo híbrido.
- Nenhum recurso atual precisa disso; este critério é só a decisão escrita.

### 4. Backend

- **`db.rs`:** pragmas de cifra explícitos; versão do esquema de cifra; `rekey` com a ordem segura (abrir com a nova antes de trocar o segredo guardado); testes de troca de chave, falha no meio e chave errada.
- **`secrets.rs`:** formato do segredo guardado com versão (chave crua ou chave envolvida pela senha), para ler os dois.
- **Senha:** Argon2id (crate `argon2`) para derivar a chave que envolve a do banco, com AES-256-GCM (crate auditado) para envolver. Sal aleatório e parâmetros guardados junto, com versão. Comandos `set_app_password`, `remove_app_password` e `unlock { password }`; a abertura passa a esperar o desbloqueio quando a senha estiver ligada.
- **Rede:** trocar `native-tls` por rustls com aws-lc-rs em `async-imap`, `lettre` e `reqwest`; conferir o tamanho do executável e o build no CI do Windows.
- **Logs:** só tipos de evento, versões e o grupo TLS negociado.

### 5. Decisões

- **D1 (pendente, recomendo oferecer, desligada por padrão):** senha do Epistula ao abrir o app.
- **D2 (pendente, recomendo sim, depois de testar o servidor):** trocar o TLS do Windows por rustls com a troca de chaves híbrida pós-quântica.
- **D3 (pendente, recomendo só manual por enquanto):** troca periódica automática da chave do banco (ex.: uma vez por ano) ou só quando o usuário pedir.
- **D4 (pendente, recomendo não):** cifrar de novo o banco com outro algoritmo "pós-quântico". Não existe cifra simétrica pós-quântica diferente do AES-256; a troca teria custo e risco sem ganho.

### 6. Riscos técnicos

- **Suporte do servidor:** o ganho do CA4 depende de o provedor aceitar o híbrido no IMAP e no SMTP; sem isso, a conexão continua clássica.
- **Certificados corporativos e proxies:** trocar o TLS do Windows pode quebrar redes que inspecionam TLS; por isso as raízes vêm do Windows.
- **Build:** aws-lc-rs precisa de CMake e NASM no Windows; conferir no CI e no script de ferramentas (`scripts/setup-build-tools.ps1`).
- **Senha esquecida:** perda das mensagens salvas no computador; precisa ficar claro antes de ativar.
- **Troca de chave interrompida:** a ordem das etapas e os testes do CA2 existem para o banco nunca ficar sem chave válida.

### 7. Fora do escopo

- E-mail cifrado entre pessoas (S/MIME, OpenPGP) e um serviço próprio de E2EE.
- Backup cifrado e sincronização entre dispositivos (quando vierem, seguem o CA5).
- Proteção contra malware rodando na sessão do usuário com o app desbloqueado.

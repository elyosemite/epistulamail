## User Story 11: Conexões com troca de chaves híbrida pós-quântica

**Como** usuário do Epistula,
**quero** que as conexões com o servidor de e-mail usem a troca de chaves híbrida pós-quântica sempre que o servidor aceitar, sem perder a conexão com servidores antigos ou em redes corporativas,
**para que** quem grava o tráfego hoje não consiga decifrá-lo quando existirem computadores quânticos.

---

### 1. Contexto

Separada da US10 em 26 out 2026. Hoje IMAP, SMTP e as chamadas de login e foto usam TLS pelo sistema operacional (`native-tls`: SChannel no Windows, Secure Transport no macOS, OpenSSL no Linux), com troca de chaves por curva elíptica. Quem gravar o tráfego hoje poderá decifrá-lo quando um computador quântico quebrar essa troca ("colher agora, decifrar depois").

A troca híbrida X25519 + ML-KEM-768 (`X25519MLKEM768`) junta a clássica com a pós-quântica padronizada (FIPS 203): a conexão só cai se as duas forem quebradas. O caminho em Rust é o rustls com o provedor aws-lc-rs. Ele também deixa o TLS igual em todos os sistemas, o que ajuda quando o app virar cross-platform.

O ganho depende do servidor: só vale onde o provedor aceita o híbrido. Os servidores web dos grandes provedores já aceitam; IMAP e SMTP ainda não se sabe.

### 2. Regras (obrigatórias)

- Nenhuma criptografia própria; só rustls e aws-lc-rs, mantidos e auditados.
- Nunca enfraquecer a proteção atual. Cair para a troca clássica é permitido, porque é o que o app usa hoje. Cair para TLS 1.0 ou 1.1, ou para cifras antigas, não é.
- Logs só com o grupo de troca de chaves negociado e o tipo de erro, nunca nada da sessão.
- Nenhum texto da interface cita o provedor.

### 3. Riscos (levantados em 26 out 2026)

1. **Servidores antigos deixam de conectar.** O rustls só aceita TLS 1.2 e 1.3 com cifras modernas (sem TLS 1.0 e 1.1, sem troca RSA, sem CBC antigo). Os grandes provedores estão bem; hospedagens pequenas, servidores próprios e Exchange antigo podem falhar. Hoje o SChannel ainda conecta em parte deles.
2. **Redes que derrubam a mensagem inicial maior.** Com o híbrido, o ClientHello passa de cerca de 300 para cerca de 1.500 bytes. Alguns firewalls, proxies corporativos e antivírus que inspecionam TLS derrubam a conexão, como aconteceu quando os navegadores ligaram o híbrido.
3. **Build mais pesado.** aws-lc-rs é C e assembly: precisa de CMake e de compilador C em todo sistema, e de NASM no Windows. Compilar para ARM, Linux, macOS e um dia celular dá mais trabalho. O SQLCipher já embute o OpenSSL; o app passa a ter duas bibliotecas de criptografia (executável maior e build mais lento).
4. **Atualizações de segurança passam a ser nossas.** Hoje uma falha no TLS é corrigida pela atualização do sistema. Com rustls, só com uma versão nova do Epistula, e ainda não existem releases assinados nem atualização automática.
5. **Certificados diferentes por sistema.** `rustls-platform-verifier` usa o verificador do sistema no Windows, no macOS e no Android (certificados corporativos e revogação continuam). No Linux usa a lista de raízes, sem a mesma checagem de revogação. Servidores com certificado autoassinado, comuns em IMAP genérico, não conectam com nenhuma das duas bibliotecas.
6. **Três bibliotecas para alinhar.** `async-imap`, `lettre` e `reqwest` precisam da mesma versão do rustls e do mesmo provedor; quando uma atualiza antes, o build quebra ou o app carrega duas versões.
7. **Ganho incerto.** Se IMAP e SMTP não aceitarem o híbrido, a troca traz os riscos acima sem proteger mais agora; só deixa pronto.

### 4. Critérios de aceite

**CA1. Medir antes**

- Conferir quais servidores (login, foto, IMAP, SMTP) do provedor atual aceitam `X25519MLKEM768`, e registrar o resultado aqui com a data.
- Repetir para cada provedor novo antes de ele entrar no app.

**CA2. Um único ponto de TLS**

- Módulo `tls.rs` que monta a configuração e entrega a conexão pronta para IMAP, SMTP e HTTP. Vale mesmo antes de trocar de biblioteca e deixa a troca num lugar só (risco 6).

**CA3. rustls com aws-lc-rs**

- `native-tls` sai de `async-imap`, `lettre` e `reqwest`; entra rustls com aws-lc-rs, oferecendo `X25519MLKEM768` primeiro e a troca clássica em seguida.
- Raízes de confiança e revogação do sistema via `rustls-platform-verifier`.
- O log registra o grupo negociado (ex.: `X25519MLKEM768` ou `X25519`).

**CA4. Redes que derrubam o híbrido (risco 2)**

- Se a conexão cair durante o handshake, o app tenta de novo só com a troca clássica, lembra disso para aquele servidor por um tempo e registra no log só o tipo do evento.

**CA5. Servidor antigo (risco 1, D2)**

- Quando o servidor só aceita TLS antigo, a conta mostra uma mensagem clara: o servidor usa uma proteção antiga que o Epistula não aceita.

**CA6. Build e CI (risco 3)**

- CI compila no Windows e também no Linux e no macOS, antes de existir a versão para esses sistemas.
- `scripts/setup-build-tools.ps1` instala e confere CMake e NASM (ou usa o NASM pronto do aws-lc-rs).
- Registrar o aumento do executável.

### 5. Decisões

- **D1 (pendente, recomendo fazer só depois do CA1 e junto com um caminho de atualização do app):** quando trocar de biblioteca. Sem releases e sem aviso de versão nova, uma falha no rustls ficaria aberta (risco 4).
- **D2 (pendente, recomendo não):** oferecer, por conta e com aviso, um modo "servidor antigo" com o TLS do sistema. Isso enfraquece a proteção daquela conta.
- **D3 (pendente, recomendo depois, numa story própria):** aceitar um certificado autoassinado guardando a impressão digital dele, com aviso forte (risco 5).

### 6. Fora do escopo

- Proteção dos dados guardados no computador (US10).
- E-mail cifrado entre pessoas.

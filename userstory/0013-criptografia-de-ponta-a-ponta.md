## User Story 13: Criptografia de ponta a ponta com OpenPGP

**Como** usuário do Epistula,
**quero** cifrar e assinar meus e-mails com OpenPGP, e gerenciar minhas chaves e as dos meus contatos num lugar só,
**para que** só o destinatário consiga ler o que eu envio (nem o provedor), e ele saiba que fui eu quem enviou e que nada foi alterado no caminho.

---

### 1. Contexto

Pedido em 26 out 2026 junto com o S/MIME; dividido em 26 out 2026 (D8): esta story traz a base e o OpenPGP, e o S/MIME está na US14 (`0014-smime.md`). Hoje o Epistula protege a cópia local (US3, US10) e o caminho até o servidor (TLS), mas **o provedor lê todos os e-mails** (ameaça 8 do modelo de ameaças). Só a criptografia de ponta a ponta muda isso.

Duas tecnologias, que não conversam entre si:

- **OpenPGP** (RFC 9580): cada pessoa gera o próprio par de chaves; a confiança vem de conferir a impressão digital (fingerprint) com a pessoa. Comum entre técnicos, jornalistas e quem usa Thunderbird, Proton ou GnuPG.
- **S/MIME** (RFC 8551, CMS RFC 5652): o certificado é emitido por uma autoridade certificadora (CA), depois de um pedido (CSR). Comum em empresas e governo; Outlook e Apple Mail já entendem.

A referência de interface é o Thunderbird: configurações de criptografia por conta, um gerenciador de chaves OpenPGP, um gerenciador de certificados S/MIME e a "aceitação" da chave de cada contato.

### 2. E a parte pós-quântica?

Aqui o risco "colher agora, decifrar depois" é maior que no TLS (US11): **o provedor guarda o e-mail cifrado para sempre**. Quem conseguir essa cópia daqui a 10 anos e tiver um computador quântico decifra o que hoje foi cifrado com X25519 ou RSA.

- **OpenPGP:** a RFC 9980 (antes `draft-ietf-openpgp-pqc`) define chaves v6 (RFC 9580) com cifra híbrida ML-KEM-768 + X25519 e assinatura híbrida ML-DSA-65 + Ed25519. O rPGP 0.21 implementa (recurso `pqc`), mas **quase nenhum cliente de e-mail lê esse formato ainda**; o GnuPG implementou uma variante própria (LibrePGP, chaves v5), que não é compatível com a da IETF.
- **S/MIME:** os padrões para ML-KEM e ML-DSA em CMS e em certificados estão saindo agora, mas **nenhuma CA emite certificado S/MIME pós-quântico**. Fica para depois, com o formato já preparado.

**Proposta (D3):** chaves OpenPGP clássicas por padrão (as que todo mundo lê) e uma chave **pós-quântica opcional, marcada como experimental**, para quem troca e-mails com quem também tem uma. Ao cifrar para alguém cuja chave tem a parte pós-quântica, o Epistula usa a híbrida sozinho. A regra do CA10 da US10 vale aqui: nada de algoritmo só pós-quântico; sempre híbrido.

### 3. As opções do Thunderbird: o que entra

| Opção do Thunderbird | No Epistula | Por quê |
|---|---|---|
| Não criptografar / criptografar mensagens novas por padrão | **Entra**, por conta | É a escolha básica. Padrão: não criptografar (D4). |
| Assinar mensagens não criptografadas | **Entra**, desligada | A assinatura sozinha vira um anexo `signature.asc` ou `smime.p7s` que confunde quem não usa criptografia. |
| Anexar minha chave pública ao assinar com OpenPGP | **Entra**, ligada | É como o contato recebe a chave para responder cifrado. |
| Enviar a chave pública no cabeçalho (Autocrypt) | **Entra**, ligada | Os clientes compatíveis passam a cifrar sem o usuário fazer nada. |
| Criptografar o assunto (OpenPGP) | **Entra**, ligada | O assunto costuma ser a parte mais reveladora; no servidor ele aparece como "...". Vale também para S/MIME (proteção de cabeçalhos), quando o destinatário entender. |
| Guardar rascunhos criptografados | **Não entra** | Os rascunhos do Epistula ficam só no banco local, que já é cifrado (US10). Volta a fazer sentido se um dia os rascunhos forem para o servidor. |
| Tecnologia preferida: automática / OpenPGP / S/MIME | **Entra** | Padrão: automática (a que todos os destinatários aceitam; empate, a última usada com eles). |

**Opções novas, que o Thunderbird não tem ou esconde:**

- **Criptografar sozinho quando todos os destinatários tiverem chave** (ligada por padrão): o cadeado liga sozinho enquanto você digita os destinatários, e avisa quando um deles não tem chave.
- **Ao responder uma mensagem criptografada, criptografar a resposta** (sempre; não é opção): responder sem cifra a uma mensagem cifrada vaza o texto citado. Se não for possível, o envio pede confirmação.
- **Avisar antes de a minha chave ou certificado vencer** (30 dias antes, no sino).

### 4. Regras (obrigatórias)

- Nenhuma criptografia própria: OpenPGP por uma biblioteca mantida e auditada (D1), S/MIME pelo OpenSSL que o app já embute (D2).
- A chave secreta nunca sai do processo Rust nem vai para log; fica no banco cifrado e, com a senha do Epistula ligada (US10), só abre desbloqueada. Exportar a chave secreta pede confirmação e sempre sai protegida por senha.
- Nada é enviado sem cifra quando o usuário pediu cifra: se algum destinatário não tem chave, o envio para e pergunta.
- Buscar chaves na internet (WKD, servidor de chaves) revela com quem você vai falar: só com confirmação ou com a opção ligada (D5).
- Logs: só tipos e resultados (assinado, cifrado, verificado, erro); nunca endereços, fingerprints, assuntos ou conteúdo.
- Toda peça nova de interface passa primeiro por `design-system/rascunho-us13.html`.
- Nenhum texto da interface cita o provedor.

### 5. Critérios de aceite

**CA1. Configurações → Criptografia (por conta)**

- Seção nova, com a conta no topo.
- **OpenPGP:** a chave usada por esta conta (lista das minhas chaves com este e-mail, ou "Nenhuma"), "Adicionar chave…" e "Gerenciar chaves OpenPGP…".
- **S/MIME:** o certificado pessoal (assinatura e cifra), "Pedir um certificado (CSR)…", "Importar certificado…" e "Gerenciar certificados S/MIME…".
- **Ao enviar:** padrão de criptografia, criptografar sozinho quando possível, assinar mensagens sem criptografia.
- **Avançado:** anexar a chave pública, Autocrypt, criptografar o assunto, tecnologia preferida, buscar chaves na internet (D5).

**CA2. Adicionar chave OpenPGP**

- Assistente com três caminhos: **criar uma chave nova**, **importar uma chave que já tenho** (arquivo `.asc`/`.gpg`/`.key`, com a senha dela) e **usar uma chave de backup do Epistula**.
- Criar: nome e e-mail da conta, validade (1, 2 ou 3 anos, ou sem validade; padrão 3 anos) e tipo: **Curve25519, recomendada**; RSA 4096, para compatibilidade antiga; **pós-quântica, experimental** (D3).
- Depois de criar: oferece fazer backup (arquivo protegido por senha) e mostra a impressão digital para compartilhar.

**CA3. Gerenciador de chaves OpenPGP**

- Lista com busca e filtro (Minhas chaves, De contatos, Todas): nome e e-mail, ID curto, criada, validade, situação (minha / aceita e verificada / aceita / ainda não aceita / rejeitada / vencida / revogada) e selo "pós-quântica".
- Detalhes de uma chave: impressão digital em grupos de 4, identidades, subchaves e algoritmos, e a **aceitação** (como o Thunderbird): "Ainda não aceitar", "Rejeitar", "Aceitar (não verifiquei)", "Aceitar (conferi a impressão digital)".
- Ações: importar (arquivo, área de transferência, buscar online), exportar a pública, backup da secreta, mudar validade, revogar (gera e guarda o certificado de revogação), excluir.
- Chaves chegam sozinhas por Autocrypt e por anexo de mensagens assinadas, sempre como "ainda não aceita"; o gerenciador avisa as novas.

**CA4 e CA5. S/MIME:** na US14.

**CA6. Escrever**

- Barra de segurança no composer: **Criptografar** (cadeado), **Assinar**, e a tecnologia em uso (OpenPGP ou S/MIME) com troca rápida.
- Cada destinatário mostra se tem chave ou certificado (cadeado) ou não (aviso).
- Cadeado ligado e algum destinatário sem chave: aviso no composer com "Buscar chaves" e "Enviar sem criptografia"; o botão Enviar não envia sem cifra sem essa escolha.
- A cópia em Enviados é cifrada também para mim (para eu conseguir ler depois).

**CA7. Ler**

- Mensagem cifrada é decifrada ao abrir; o conteúdo decifrado não vai para o servidor.
- Selo de segurança no cabeçalho da mensagem: "Criptografada", "Assinada por Ana Ribeiro", "Assinatura verificada" (chave aceita e conferida), com detalhes num painel (tecnologia, algoritmos, chave ou certificado, data da assinatura).
- Problemas em destaque: assinatura inválida (a mensagem pode ter sido alterada), chave do remetente ainda não aceita, certificado vencido ou não confiável, e "nenhuma das suas chaves abre esta mensagem".
- Assunto cifrado: mostra o assunto verdadeiro depois de decifrar (D7 decide o que a lista mostra).

**CA8. Avisos**

- Minha chave ou certificado vence em 30 dias: aviso no sino e em Configurações → Criptografia.
- Chave nova de um contato chegou (Autocrypt ou anexo): aviso discreto no gerenciador.
- Um contato trocou de chave: aviso ao escrever para ele (pode ser legítimo ou um ataque).

### 6. Backend

- **Biblioteca OpenPGP (D1):** rPGP (`pgp`, Rust puro, MIT/Apache, auditada) ou Sequoia (`sequoia-openpgp`, LGPL). Conferir antes: suporte a RFC 9580 (v6), ao rascunho pós-quântico e a licença com o Epistula.
- **S/MIME (D2):** crate `openssl` usando o OpenSSL que o SQLCipher já compila (CMS, PKCS#12, X.509, CSR, verificação de cadeia). O OpenSSL 3.5 já traz ML-KEM e ML-DSA, o que deixa o caminho pós-quântico aberto para quando as CAs emitirem.
- **Migração:** `pgp_key` (fingerprint, pública, secreta cifrada quando minha, identidades, validade, situação, aceitação, origem, pós-quântica), `smime_cert` (DER, chave privada quando minha, titular, e-mails, emissor, validade, uso, confiança), `smime_request` (chave privada + CSR pendente), `account_crypto` (configurações por conta).
- **Chaves secretas:** guardadas cifradas pela chave do banco; com a senha do Epistula ligada, também ficam fechadas com o app bloqueado (US10).
- **MIME:** PGP/MIME (RFC 3156) para OpenPGP, `application/pkcs7-mime` e `multipart/signed` para S/MIME; proteção de cabeçalhos (assunto) nos dois.
- **Envio:** `send.rs` monta a mensagem, assina e cifra antes do SMTP; a cifra inclui a chave do próprio remetente.
- **Leitura:** `message.rs` decifra e verifica antes de sanitizar; o HTML decifrado passa pelo mesmo sanitizador.
- **Busca de chaves (D5):** WKD (domínio do destinatário) e `keys.openpgp.org`, só quando permitido.
- **Comandos:** gerar, importar, exportar, aceitar, revogar, excluir chaves; gerar CSR, importar certificado e PKCS#12; listar; configurações por conta; estado de chaves dos destinatários.

### 7. Decisões

- **D1 (aprovada em 26 out 2026: rPGP depois de conferir):** biblioteca OpenPGP. rPGP é Rust puro e MIT/Apache; Sequoia é a mais completa, mas LGPL, o que exige cuidado na distribuição.
- **D2 (aprovada em 26 out 2026: o OpenSSL já embutido):** S/MIME por OpenSSL (maduro, já compilado no app) ou pelos crates RustCrypto (`cms`, `x509-cert`; mais novos).
- **D3 (aprovada em 26 out 2026: clássica por padrão e pós-quântica opcional e experimental):** tipo da chave OpenPGP nova.
- **D4 (aprovada em 26 out 2026: não criptografar por padrão e criptografar sozinho quando todos tiverem chave):** padrão de envio.
- **D5 (aprovada em 26 out 2026: perguntar a cada busca):** buscar chaves na internet (WKD, servidor de chaves), que revela a terceiros com quem você vai falar.
- **D6 (aprovada em 26 out 2026: as raízes do Windows):** onde estão as CAs confiáveis para S/MIME.
- **D7 (aprovada em 26 out 2026: guardar o assunto decifrado só no banco local):** o que a lista mostra de uma mensagem com assunto cifrado. Decifrar ao sincronizar exige a chave secreta (e o app desbloqueado); sem isso, a lista mostra "Mensagem criptografada".
- **D8 (aprovada em 26 out 2026: duas stories):** dividir em US13 (base e OpenPGP) e US14 (S/MIME), ou entregar tudo junto.

### 8. Riscos técnicos

- **Compatibilidade:** OpenPGP tem várias versões (v4, v5 do GnuPG, v6 da IETF); S/MIME depende das CAs e de como cada cliente monta o MIME. Testar contra Thunderbird, GnuPG/Kleopatra, Outlook e Apple Mail.
- **Chave perdida:** sem backup, as mensagens cifradas para ela não abrem nunca mais. O assistente insiste no backup.
- **Busca e índice:** o corpo cifrado não entra na busca nem na prévia (o trecho fica "Mensagem criptografada").
- **Regras (US12):** condições de texto não enxergam o conteúdo cifrado; só cabeçalhos.
- **Ataques conhecidos:** EFAIL (conteúdo externo em mensagens decifradas): o bloqueio de imagens externas e o sanitizador já cortam esse caminho, e partes cifradas e não cifradas misturadas não são juntadas na exibição.
- **Pós-quântica:** formato ainda em rascunho na IETF; pode mudar antes de virar RFC.

### 9. Implementação (26 out 2026)

- **Backend:** rPGP 0.21 com a feature `pqc` (D1). `openpgp.rs` gera as chaves (Curve25519 v4, RSA 4096 e pós-quântica v6 com ML-KEM-768 + X25519 e ML-DSA-65 + Ed25519, pela RFC 9980), importa, exporta, cifra (SEIPDv2 com OCB quando todas as chaves são v6, senão SEIPDv1; AES-256), assina, verifica e decifra. `pgp_mime.rs` monta e lê PGP/MIME (RFC 3156), o assunto protegido (protected-headers v1, o servidor vê "...") e o cabeçalho Autocrypt. `pgp_store.rs` guarda as chaves e as configurações por conta (migração 0009). `e2ee.rs` abre a mensagem na leitura e escolhe as chaves do envio. `key_lookup.rs` busca por WKD (métodos avançado e direto) e em `keys.openpgp.org`.
- **Envio:** com o cadeado ligado, a mensagem é cifrada para os destinatários e para a minha chave e sempre assinada; sem ele, pode sair só assinada (`multipart/signed`). Destinatário sem chave aceita recusa o envio antes de agendar (`MissingRecipientKey`).
- **Leitura:** decifra e verifica antes de sanitizar; o HTML decifrado passa pelo mesmo sanitizador e pelo bloqueio de imagens externas. Chaves que chegam por Autocrypt ou anexo `application/pgp-keys` entram como "ainda não aceita".
- **Frontend:** Configurações → Criptografia (`features/crypto/CryptoSettings.tsx`): chave da conta (L64, L65), "Ao enviar" e "Avançado". Assistente "Adicionar chave" (criar, importar, restaurar backup) com a impressão digital e o backup protegido por senha (`AddKeyDialog.tsx`). Gerenciador de chaves numa folha própria com "Voltar" (`KeyManager.tsx`, L66): busca, filtro, importar de arquivo, da área de transferência ou da internet, aceitação no estilo do Thunderbird, exportar a pública (L69), backup da secreta e excluir. Composer: faixa de segurança (L68), cadeado nos chips, aviso de destinatário sem chave com "Buscar a chave na internet" (o clique é o consentimento, D5), "Aceitar a chave encontrada", "Enviar sem criptografia" e "Tirar este destinatário", e o rótulo do Enviar. A resposta a uma mensagem criptografada começa criptografada. Leitura: selos (L67) e o painel "Segurança desta mensagem", com avisos de assinatura inválida e de mensagem que nenhuma chave abre.
- **Arquivos:** salvar vai para a pasta Downloads pelo próprio WebView (sem plugin de diálogo); abrir usa o seletor do sistema.
- **Limitações desta entrega:**
  - O rPGP não define validade ao gerar: as chaves novas saem sem validade, o assistente não pergunta, e "Mudar validade…" e "Revogar…" ainda não existem.
  - O cadeado na lista (`message.crypto`) e o assunto decifrado na lista (D7) ainda não são preenchidos na sincronização; a lista mostra o que o servidor tem ("..." para assunto protegido).
  - Chaves por Autocrypt só entram ao abrir a mensagem, não ao sincronizar.
  - "Tecnologia preferida" fica escondida até a US14 (S/MIME), porque hoje só há OpenPGP.
  - O aviso de "contato trocou de chave" ainda não existe.
- **Testes:** 174 no Rust e 88 no frontend passam; clippy, typecheck e build sem avisos.
- **Falta (manual):** trocar mensagens com o Thunderbird e o Kleopatra (cifrada, assinada, Autocrypt e chave anexada), restaurar um backup em outro computador, a busca WKD num domínio real, e conferir que salvar arquivo e ler a área de transferência funcionam no WebView2.

### 10. Fora do escopo

- Smartcards e tokens (YubiKey, cartão ICP-Brasil em hardware).
- Assinatura de mensagens com certificado ICP-Brasil para validade jurídica.
- Teia de confiança (assinar chaves de outros).
- Servidor de chaves próprio.

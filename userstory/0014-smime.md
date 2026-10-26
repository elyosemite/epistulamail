## User Story 14: Criptografia de ponta a ponta com S/MIME

**Como** usuário do Epistula,
**quero** pedir, importar e usar um certificado S/MIME para assinar e cifrar meus e-mails, e guardar os certificados dos meus contatos,
**para que** eu troque mensagens cifradas e assinadas com quem usa Outlook, Apple Mail ou o certificado da empresa.

---

### 1. Contexto

Separada da US13 em 26 out 2026 (D8 da US13). A US13 traz a base da criptografia de ponta a ponta (configurações por conta, barra de segurança no composer, selos na leitura) e o OpenPGP; esta story acrescenta o S/MIME nas mesmas peças. O rascunho de interface aprovado é o da US13 (`design-system/rascunho-us13.html`, seção S/MIME).

Decisões já aprovadas na US13 que valem aqui: **D2** (S/MIME pelo OpenSSL que o app já embute) e **D6** (raízes de confiança do Windows). Pós-quântico: nenhuma CA emite certificado S/MIME com ML-DSA ou ML-KEM ainda; o OpenSSL 3.5 já tem os algoritmos, e o formato guardado fica pronto para eles.

### 2. Como testar sem comprar certificado

- **CA de teste local:** `scripts/smime-test-ca.ps1` cria uma CA de teste com o `openssl` que vem com o Git para Windows e assina o CSR gerado pelo Epistula. Para outros programas confiarem, a raiz de teste é importada só no usuário atual do Windows (dá para remover depois).
- **Certificado gratuito:** a Actalis emite certificados S/MIME gratuitos de 1 ano (a conferir na época); costuma entregar `.p12`, o que testa a importação.
- **Testes automáticos:** os testes do Rust criam CA e certificados na hora, sem rede.

### 3. Critérios de aceite

**CA1. S/MIME: pedir um certificado (CSR)**

- Assistente: e-mail da conta (fixo), nome, organização (opcional), país; algoritmo **RSA 3072, recomendado** (o que as CAs mais aceitam) ou ECDSA P-256.
- O par de chaves é gerado no computador; o CSR (PEM) aparece para copiar ou salvar, com a explicação de onde levar.
- O pedido fica "aguardando certificado" no gerenciador. "Importar certificado emitido" (`.cer`, `.crt`, `.pem`, `.p7b`) casa o certificado com a chave guardada e confere se o e-mail bate.
- Também importa um certificado completo (`.p12`/`.pfx`, com a senha).

**CA2. Gerenciador de certificados S/MIME**

- Abas: **Meus certificados**, **Pessoas** (recolhidos das mensagens assinadas) e **Pedidos pendentes**.
- Detalhes: titular, e-mail, emissor, validade, uso (assinatura/cifra), impressão digital SHA-256, cadeia até a raiz e se ela é confiável (raízes do Windows, D6).
- Ações: exportar (público `.cer`; o meu com chave, `.p12` com senha), excluir, renovar (novo CSR com os mesmos dados).

**CA3. Escrever e ler**

- A barra de segurança do composer (US13) passa a oferecer S/MIME; a escolha automática segue a tecnologia preferida da conta.
- `application/pkcs7-mime` (cifrada) e `multipart/signed` com `application/pkcs7-signature` (assinada), com a cópia cifrada também para o remetente.
- Certificados de quem assina chegam sozinhos ao abrir mensagens assinadas (aba Pessoas).
- Os selos de leitura (US13) cobrem S/MIME: cadeia confiável, certificado vencido, revogado ou de outro e-mail.

### 4. Backend

- `openssl` (crate) com o OpenSSL embutido: CSR (X509Req), PKCS#12, CMS sign/encrypt/decrypt/verify, verificação de cadeia com as raízes do Windows.
- Migração: `smime_cert`, `smime_request`.

### 5. Decisões

- **D1 (pendente, recomendo não consultar OCSP e CRL por padrão):** conferir revogação na hora de ler revela a terceiros quando você abriu a mensagem. Alternativa: perguntar ou consultar em segundo plano. *Implementado como a recomendação (nada é consultado) até a decisão.*
- **D2 (pendente, recomendo manter o assunto aberto no S/MIME):** o assunto protegido (cabeçalhos dentro da parte cifrada) é lido pelo Thunderbird, mas não pelo Outlook nem pelo Apple Mail, onde o assunto apareceria como "...". Alternativa: seguir a opção "Criptografar também o assunto" também no S/MIME. *Implementado como a recomendação: com S/MIME o assunto vai aberto; a opção vale só para o OpenPGP, e a tela diz isso.*

### 6. Implementação (26 out 2026)

- **Backend:** crate `openssl` com o OpenSSL que o SQLCipher já embute (D2 da US13), mais `openssl-sys` para duas funções do CMS que o crate não expõe (`CMS_get0_signers`, `CMS_get1_certs`) e `schannel` para ler as raízes do usuário no Windows (D6). `smime.rs`: CSR (RSA 3072 ou ECDSA P-256, com e-mail no SAN, uso de assinatura e cifra e proteção de e-mail), leitura de PEM, DER e `.p7b`, PKCS#12 (ler e exportar, senha de 8 ou mais), dados do certificado (SHA-256, titular, emissor, uso, validade), cadeia até as raízes, CMS (cifrar com AES-256-CBC, assinar com SHA-256, verificar, decifrar). `smime_mime.rs`: `application/pkcs7-mime` e `multipart/signed` com `application/pkcs7-signature`. `smime_store.rs` e a migração 0010 (`smime_cert`, `smime_cert_email`, `smime_request`); `CryptoSettings.smimeCert`.
- **Pedido (CA1):** o par de chaves nasce no computador; o certificado importado depois casa com o pedido pela chave pública e precisa ter o e-mail do pedido (`CertificateEmailMismatch`). O `.p12` entra com a senha. O certificado meu importado vira o da conta do mesmo e-mail que ainda não tem um.
- **Envio (CA3):** sempre assinado quando cifrado: a parte `multipart/signed` vai dentro do envelope (como fazem Outlook e Thunderbird), cifrado também para o meu certificado. Só cifra para certificados com uso de cifra, dentro da validade e com a cadeia confiável (`MissingRecipientCertificate` antes de agendar).
- **Leitura (CA3):** envelope, assinatura destacada e assinatura opaca (`signed-data`), também a assinatura dentro do envelope. O certificado de quem assinou vai para a aba Pessoas só se for do endereço do De. Selos: verde só com cadeia confiável e o e-mail certo; vencido, emissor não confiável e certificado de outro e-mail em amarelo.
- **Frontend:** grupo S/MIME em Configurações → Criptografia (certificado pessoal, pedir CSR, importar, gerenciar) e "Tecnologia preferida". Gerenciador de certificados numa folha própria (Meus, Pessoas, Pedidos pendentes; detalhes com cadeia, uso e SHA-256; exportar `.cer`, backup `.p12`, renovar, excluir). No composer, a tecnologia sai da conta e dos destinatários (`pickTechnology`) e "Trocar" alterna quando a conta tem as duas.
- **CA de teste:** `scripts/smime-test-ca.ps1` cria a CA em `%LOCALAPPDATA%\Epistula\test-ca`, assina o `.csr` salvo no Epistula, cria contatos de teste (`.p12`) e põe ou tira a raiz de teste no usuário do Windows.
- **Limitações desta entrega:**
  - Revogação (OCSP, CRL) não é consultada (D1); um certificado revogado aparece como confiável.
  - O Windows baixa algumas raízes públicas só quando precisa delas; uma CA cuja raiz ainda não veio para o computador aparece como "não confiável". As raízes são lidas uma vez por execução.
  - AES-256-CBC e o transporte de chave RSA padrão do OpenSSL (PKCS#1 v1.5), pela compatibilidade; AES-GCM (AuthEnvelopedData) não é enviado; a leitura passa pelo OpenSSL, que o aceita, mas não tem teste.
  - Os certificados de quem assina só entram ao abrir a mensagem; o cadeado na lista (`message.crypto`) continua vazio.
  - "Renovar" refaz o pedido com o nome, a organização e o algoritmo do certificado; o país volta ao padrão.
  - Quando todos os destinatários têm chave e certificado, o "automático" escolhe OpenPGP, e não a tecnologia usada por último com eles (seção 3 da US13).
- **Testes:** 187 no Rust (CA, emissão, cadeia, PKCS#12, CMS com RSA e ECDSA, MIME depois do transporte, importação com pedido, envio e leitura completos) e 93 no frontend passam; clippy, typecheck e build sem avisos.
- **Falta (manual):** trocar mensagens com o Outlook e o Thunderbird usando a CA de teste e o certificado gratuito da Actalis; conferir a pasta Downloads para `.csr`, `.cer` e `.p12` no WebView2.

### 7. Fora do escopo

- Smartcards e certificados ICP-Brasil em hardware.

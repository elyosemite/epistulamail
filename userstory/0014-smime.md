## User Story 14: Criptografia de ponta a ponta com S/MIME

**Como** usuário do Epistula,
**quero** pedir, importar e usar um certificado S/MIME para assinar e cifrar meus e-mails, e guardar os certificados dos meus contatos,
**para que** eu troque mensagens cifradas e assinadas com quem usa Outlook, Apple Mail ou o certificado da empresa.

---

### 1. Contexto

Separada da US13 em 26 out 2026 (D8 da US13). A US13 traz a base da criptografia de ponta a ponta (configurações por conta, barra de segurança no composer, selos na leitura) e o OpenPGP; esta story acrescenta o S/MIME nas mesmas peças. O rascunho de interface aprovado é o da US13 (`design-system/rascunho-us13.html`, seção S/MIME).

Decisões já aprovadas na US13 que valem aqui: **D2** (S/MIME pelo OpenSSL que o app já embute) e **D6** (raízes de confiança do Windows). Pós-quântico: nenhuma CA emite certificado S/MIME com ML-DSA ou ML-KEM ainda; o OpenSSL 3.5 já tem os algoritmos, e o formato guardado fica pronto para eles.

### 2. Como testar sem comprar certificado

- **CA de teste local:** `scripts/smime-test-ca.ps1` (a criar) cria uma CA de teste com o `openssl` que vem com o Git para Windows e assina o CSR gerado pelo Epistula. Para outros programas confiarem, a raiz de teste é importada só no usuário atual do Windows (dá para remover depois).
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

- **D1 (pendente, recomendo não consultar OCSP e CRL por padrão):** conferir revogação na hora de ler revela a terceiros quando você abriu a mensagem. Alternativa: perguntar ou consultar em segundo plano.

### 6. Fora do escopo

- Smartcards e certificados ICP-Brasil em hardware.

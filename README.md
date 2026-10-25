# Epistula

Um cliente de e-mail para desktop que guarda as suas mensagens no seu computador, cifradas, e não depende de nenhum servidor nosso.

O Epistula conecta as suas contas, baixa o que você precisa ler e mantém tudo num banco local protegido por AES-256. A chave fica no cofre do sistema operacional. Não existe conta Epistula, não existe nuvem Epistula e não existe ninguém do nosso lado com acesso ao que você recebe.

![Caixa de entrada do Epistula com várias contas, pastas, lista de mensagens e uma mensagem aberta com as imagens externas bloqueadas](docs/images/epistula-caixa-de-entrada.png)

> O projeto está em desenvolvimento ativo. Leitura, organização e múltiplas contas já funcionam. Envio, backup completo e criptografia pós-quântica estão a caminho. A seção [Estado atual](#estado-atual) diz exatamente o que existe hoje.

## Por que mais um cliente de e-mail

O e-mail é o arquivo mais longo da vida digital de quase todo mundo. Contratos, exames, conversas de família, recuperação de senha de tudo. E esse arquivo costuma morar em um único lugar, controlado por outra empresa, acessível por um navegador.

O Epistula parte de três ideias simples:

**Seus dados ficam com você.** Cada mensagem que você abre é guardada no seu disco. Se a rede cair, a caixa continua lá. Se o provedor mudar de regra, a sua cópia continua sua.

**O que fica no disco fica cifrado.** O banco inteiro é cifrado, inclusive índices, assuntos e remetentes. Um backup do arquivo, um notebook esquecido no táxi ou um HD descartado não revelam nada sem a chave.

**Segurança se demonstra, não se promete.** O modelo de ameaças está publicado, os riscos que ainda não resolvemos estão escritos, e as garantias que dá para testar têm teste.

## Estado atual

O que funciona hoje:

- Conexão com contas Google por OAuth (PKCE), lendo a caixa por IMAP sobre TLS. Nenhuma senha passa pelo app.
- Várias contas ao mesmo tempo, cada uma com nome, cor e foto, e troca com um clique.
- Pastas e marcadores, paginação e sincronização em segundo plano com intervalo configurável por conta.
- Banco local SQLCipher (AES-256, HMAC por página). A lista abre do disco e continua disponível offline.
- Leitura segura: HTML de terceiros limpo por allowlist no processo Rust e exibido num iframe sem scripts. Imagens remotas ficam bloqueadas até você liberar, mensagem por mensagem.
- Mover, arquivar, excluir, marcar como lida e sinalizar com prazo, inclusive em várias mensagens de uma vez e arrastando para a pasta.
- Busca, barra de comandos e rascunhos locais de resposta, resposta a todos e encaminhamento.
- Tema claro, escuro ou do sistema, e cor de fundo personalizável.

O que ainda não existe:

- Envio de mensagens (SMTP). Os rascunhos já ficam salvos, cifrados.
- Outros provedores além do Google. O modelo de dados já prevê Outlook, Yahoo, iCloud, Zoho, GMX, AOL e IMAP genérico.
- Releases assinados. Por enquanto, o caminho é compilar a partir do código.

## Para onde vamos

### Criptografia pós-quântica

Mensagens cifradas hoje podem ser capturadas hoje e decifradas daqui a alguns anos, quando houver computadores quânticos capazes de quebrar as trocas de chave atuais. Para e-mail, que costuma ser guardado por décadas, esse risco não é teórico.

A cifra simétrica que o Epistula já usa em repouso (AES-256) continua forte nesse cenário. O ponto frágil está em tudo que envolve chave pública: backups que precisam ser abertos em outro computador, sincronização entre dispositivos seus e mensagens cifradas de ponta a ponta. É ali que entram esquemas híbridos, combinando um algoritmo clássico com um pós-quântico padronizado (X25519 com ML-KEM), para que a proteção não dependa de uma única aposta.

### Backup que você controla

Exportar uma conta inteira para um arquivo cifrado, guardar onde quiser (um disco externo, um pendrive, o serviço de armazenamento da sua escolha) e restaurar em outra máquina sem precisar baixar tudo de novo do provedor. O backup deve ser legível só por quem tem a chave, e o formato deve ser documentado, para que você nunca fique preso ao Epistula.

### Gestão de e-mail de verdade

Busca local completa, mesmo offline. Regras e filtros. Adiar mensagens. Visão unificada de todas as contas. Assinaturas, modelos e agendamento de envio. Indicadores de autenticidade do remetente (SPF, DKIM, DMARC) visíveis na leitura, para ajudar a reconhecer phishing.

## Como a sua informação é tratada

```
Seu computador
├── Interface (WebView)      só exibe; nunca vê tokens nem a chave do banco
├── Processo Rust            OAuth, IMAP, limpeza de HTML, decide o que sai para a tela
├── Cofre do sistema         refresh tokens e a chave do banco
└── epistula.db              mensagens, pastas e ajustes, cifrado por inteiro

        │ TLS
        ▼
Seu provedor de e-mail
```

Alguns pontos que valem ser ditos com clareza:

- **O seu provedor continua lendo os seus e-mails.** O Epistula protege a cópia local e o caminho até ela. Enquanto não houver criptografia de ponta a ponta, quem hospeda a caixa tem acesso a ela.
- **Os logs não contêm conteúdo.** Nem assunto, nem remetente, nem endereço da conta. Um teste percorre o app inteiro com marcadores únicos e falha se algum aparecer no log ou em texto puro no arquivo do banco.
- **Sair de uma conta revoga o acesso** no provedor e apaga o segredo do cofre do sistema. As mensagens já baixadas continuam no disco, cifradas, até que exista a opção de apagá-las.
- **Um computador comprometido continua comprometido.** Malware rodando na sua sessão consegue ler o que você consegue ler. Nenhum cliente de e-mail resolve isso, e preferimos dizer isso a fingir o contrário.

A análise completa, com a matriz de dados, os limites de confiança e as doze ameaças consideradas, está em [docs/arquitetura-e-ameacas.md](docs/arquitetura-e-ameacas.md).

## Compilando

O alvo principal hoje é o Windows. O Epistula é feito com [Tauri 2](https://tauri.app), Rust e React.

Requisitos:

- Node.js 24 e pnpm 10
- Rust 1.82 ou mais recente
- No Windows, o Perl portátil usado para compilar o OpenSSL embutido do SQLCipher. O script abaixo baixa e confere o SHA-256.

```powershell
pnpm install
./scripts/setup-build-tools.ps1
```

Para conectar uma conta Google, crie um cliente OAuth do tipo "Desktop app" no Google Cloud e preencha o `.env`:

```powershell
Copy-Item .env.example .env
# EPISTULA_GOOGLE_CLIENT_ID=...
# EPISTULA_GOOGLE_CLIENT_SECRET=...
```

Depois:

```powershell
pnpm tauri dev      # desenvolvimento
pnpm tauri build    # instalador
```

## Testes

```powershell
pnpm test                         # interface
cd src-tauri; cargo test          # backend, com banco SQLCipher real
cargo clippy --all-targets -- -D warnings
```

O CI roda tudo isso no Windows a cada push, além de uma varredura de segredos no histórico do repositório.

## Estrutura

```
src/                 interface em React
src-tauri/src/       backend em Rust (OAuth, IMAP, sincronização, banco, limpeza de HTML)
src-tauri/migrations versões do esquema do banco
design-system/       componentes e tokens visuais
docs/                arquitetura e modelo de ameaças
userstory/           histórico de decisões, uma story por entrega
```

## Contribuindo

Issues e pull requests são bem-vindos. Antes de abrir um PR que toque em credenciais, armazenamento ou exibição de conteúdo de terceiros, leia o modelo de ameaças: mudanças nessas áreas precisam dizer qual ameaça afetam e como foram testadas.

Encontrou uma falha de segurança? Não abra uma issue pública. Escreva para os mantenedores pelo GitHub em privado e daremos retorno antes de qualquer divulgação.

## Licença

A definir.

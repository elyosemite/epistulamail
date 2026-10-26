# Epistula

[English](README.md) | **Português**

Um cliente de e-mail para desktop que guarda as suas mensagens no seu computador, cifradas, e não depende de nenhum servidor nosso.

O Epistula conecta as suas contas, baixa o que você precisa ler e mantém tudo num banco local protegido por AES-256. A chave fica no cofre do sistema operacional. Não existe conta Epistula, não existe nuvem Epistula e não existe ninguém do nosso lado com acesso ao que você recebe.

![Caixa de entrada do Epistula com três contas, pastas e marcadores, a lista de mensagens e uma newsletter aberta no painel de leitura](docs/images/epistula-caixa-de-entrada.png)

> O projeto está em desenvolvimento ativo. Leitura, envio, organização, regras, conversas, notificações, leitura sem conexão e várias contas já funcionam. Senha do app, bloqueio automático e backup cifrado estão a caminho. A seção [Estado atual](#estado-atual) diz exatamente o que existe hoje.

## Em poucas palavras

- **Sem conta Epistula e sem nuvem Epistula.** O app fala direto com o seu provedor. Não há servidor nosso no meio.
- **Tudo cifrado no disco.** Mensagens, assuntos, remetentes e índices ficam num banco AES-256, que já resiste a computadores quânticos.
- **Abre na hora, até sem internet.** Os e-mails recentes são baixados em segundo plano e abrem do disco.
- **Contra rastreamento.** Imagens externas (os "pixels espiões" que avisam que você abriu) ficam bloqueadas até você liberar.
- **Conversas fáceis de ler.** Cada resposta num card, com o texto citado escondido.
- **Se organiza sozinho.** Regras movem, arquivam ou sinalizam os e-mails assim que chegam, com prévia antes de salvar e desfazer depois.
- **Avisa só o que importa.** Notificações do Windows, bandeja do sistema, horário de silêncio e um histórico no sino.
- **Segurança que dá para conferir.** Modelo de ameaças público e testes que falham se algum conteúdo aparecer nos logs.

## Por que mais um cliente de e-mail

O e-mail é o arquivo mais longo da vida digital de quase todo mundo. Contratos, exames, conversas de família, recuperação de senha de tudo. E esse arquivo costuma morar em um único lugar, controlado por outra empresa, acessível por um navegador.

O Epistula parte de três ideias simples:

**Seus dados ficam com você.** Cada mensagem que você abre é guardada no seu disco. Se a rede cair, a caixa continua lá. Se o provedor mudar de regra, a sua cópia continua sua.

**O que fica no disco fica cifrado.** O banco inteiro é cifrado, inclusive índices, assuntos e remetentes. Um backup do arquivo, um notebook esquecido no táxi ou um HD descartado não revelam nada sem a chave.

**Segurança se demonstra, não se promete.** O modelo de ameaças está publicado, os riscos que ainda não resolvemos estão escritos, e as garantias que dá para testar têm teste.

## Estado atual

O que funciona hoje:

**Contas e segurança**

- Conexão com contas Google por OAuth (PKCE), lendo por IMAP e enviando por SMTP, sempre sobre TLS. Nenhuma senha passa pelo app.
- Várias contas ao mesmo tempo, cada uma com nome, cor e foto, e troca com um clique.
- Banco local SQLCipher (AES-256, HMAC por página), com a chave no Cofre de Credenciais do Windows e exclusão segura das páginas apagadas.
- Leitura segura: o HTML de terceiros é limpo por allowlist no processo Rust e exibido num iframe sem scripts. As imagens externas ficam bloqueadas até você liberar, mensagem por mensagem.
- Notificações com privacidade: você escolhe se elas mostram remetente e assunto ou só a conta.

**Leitura**

- Conversas em cards: cada resposta separada, as não lidas abertas e o texto citado escondido atrás de um "…".
- Leitura sem conexão: depois de cada sincronização, os e-mails do período escolhido (30 dias, 90 dias, tudo ou só ao abrir) são baixados em segundo plano, sem marcar como lidos.
- Abrir é instantâneo: o app já busca as próximas mensagens da lista, as da conversa e a que está sob o mouse.
- Primeira sincronização em etapas: a lista aparece com as 200 mais recentes, e o resto chega aos poucos.

**Escrita e organização**

- Escrever, responder, responder a todos e encaminhar, com rascunhos salvos cifrados.
- Pastas e marcadores, sinalizadas com prazo, e mover, arquivar, excluir e marcar como lida, em uma ou várias mensagens, inclusive arrastando para a pasta.

**Regras**

- Regras que organizam os e-mails assim que chegam, antes de virarem notificação. Cada regra se lê como uma frase: "Se De contém @loja.com → Mover para Compras".
- Condições por remetente, domínio, Para e Cc, "enviado só para mim", assunto, trecho do texto, lista de e-mails, anexos, tamanho, lida e estrela, com "todas" ou "qualquer uma" e exceções ("exceto se…"). Maiúsculas e acentos não fazem diferença, e vários valores separados por vírgula valem como "qualquer um".
- Ações: mover, arquivar, excluir (sempre para a Lixeira), aplicar marcador, marcar como lida, estrela, sinalizar com prazo, não notificar e notificar sempre (até no Não perturbe). Ordem entre as regras e "parar de aplicar as outras".
- Prévia ao vivo no editor: antes de salvar, você vê quantos e quais e-mails a regra pegaria, e o aviso quando outra regra acima a impede de rodar.
- Tudo pode ser desfeito: o histórico guarda 30 dias do que cada regra fez, com "Desfazer" em cada lote, e um aviso depois de cada sincronização mostra o que foi organizado.
- Comece por um modelo (Newsletters, Avisos automáticos, Recibos e faturas, Pessoas importantes) ou direto de uma mensagem: "Criar regra…", "Sempre mover deste remetente para…" e "Excluir tudo deste remetente…".
- "Rodar agora" aplica uma regra aos e-mails que já estão na pasta, e regras de limpeza ("mais antigo que 30 dias", "manter só o mais recente do remetente") rodam uma vez por dia.
- As regras rodam no Epistula, com o app aberto (inclusive na bandeja), e valem para qualquer provedor. O que chegou com o app fechado passa por elas na sincronização seguinte.

**Criptografia de ponta a ponta**

- OpenPGP embutido: crie uma chave (Curve25519, RSA 4096 ou uma pós-quântica experimental, com ML-KEM-768 + X25519), importe uma que você já tem ou restaure um backup do Epistula.
- Um cadeado no composer criptografa e assina a mensagem. Ele liga sozinho quando todos os destinatários têm chave aceita, e respostas a mensagens criptografadas já começam criptografadas. Dá para esconder também o assunto do servidor.
- Quem não tem chave aparece antes do envio, com a escolha de buscar a chave na internet (só quando você manda), enviar sem criptografia ou tirar o destinatário.
- A leitura mostra se a mensagem veio criptografada e se a assinatura confere, com a aceitação da chave no estilo do Thunderbird (ainda não, aceita, verificada, rejeitada).
- Um gerenciador de chaves com busca, importação de arquivo, da área de transferência ou da internet, exportação e backup protegido por senha. Chaves que chegam por Autocrypt ou anexadas ficam como "ainda não aceita".
- S/MIME também, para quem usa Outlook, Apple Mail ou o certificado da empresa: peça um certificado (o par de chaves nasce no seu computador e só o pedido vai para a autoridade certificadora), importe o emitido ou um `.p12` e guarde os certificados de quem manda mensagens assinadas. A confiança segue as raízes do Windows. Um script de CA de teste deixa experimentar sem comprar nada.

**Avisos**

- Notificações do Windows para e-mails novos, respostas aos seus e-mails, prazos e contas que precisam de atenção. Um clique abre a mensagem.
- O app continua na bandeja do sistema com a janela fechada, com "novo e-mail" e silenciar por um tempo.
- Horário de silêncio e um sino com o histórico dos últimos 7 dias.

**Aparência**

- Tema claro, escuro ou do sistema, cor de fundo personalizável, densidade da lista e colunas que se recolhem para focar na leitura.

O que ainda não existe:

- Outros provedores além do Google. O modelo de dados já prevê Outlook, Yahoo, iCloud, Zoho, GMX, AOL e IMAP genérico.
- Releases assinados. Por enquanto, o caminho é compilar a partir do código.

## Para onde vamos

### Seus dados só seus, mesmo no seu computador

Hoje o banco é cifrado contra quem está fora do seu Windows. O próximo passo é protegê-lo também de quem está dentro dele: outra pessoa na sua sessão, o administrador de um domínio corporativo ou alguém com uma cópia do disco e a sua senha do Windows.

- **Senha do Epistula (opcional):** a chave do banco passa a ser protegida por uma chave derivada da sua senha com Argon2id. A senha não fica gravada em lugar nenhum.
- **Bloqueio automático:** depois de um tempo sem uso ou quando o Windows bloqueia a tela, o app fecha o banco e apaga a chave da memória.
- **Nada vaza para fora do banco:** notificações sem conteúdo por padrão, nenhum e-mail no cache do navegador interno, nenhum despejo de memória em disco após uma falha, e um aviso quando o disco não tem BitLocker.
- **Guardar menos:** apagar do computador os e-mails fora do período escolhido e os de contas que você deixou, trocando a chave do banco em seguida.
- **Links honestos:** aviso quando o link leva a um endereço diferente do que aparece, e remoção dos parâmetros de rastreamento.

### Pronto para computadores quânticos

Mensagens cifradas hoje podem ser capturadas hoje e decifradas daqui a alguns anos, quando houver computadores quânticos capazes de quebrar as trocas de chave atuais. Para e-mail, que costuma ser guardado por décadas, esse risco não é teórico.

A cifra que o Epistula já usa no disco (AES-256) continua forte nesse cenário; não é preciso trocá-la. No caminho até o provedor, o app vai oferecer a troca de chaves híbrida (X25519 com ML-KEM-768), que passa a valer assim que o servidor aceitar. E tudo que vier a usar chave pública (backup aberto em outro computador, sincronização entre dispositivos seus, mensagens cifradas de ponta a ponta) já nasce com esquemas híbridos, para que a proteção não dependa de uma única aposta.

### Backup que você controla

Exportar uma conta inteira para um arquivo cifrado, guardar onde quiser (um disco externo, um pendrive, o serviço de armazenamento da sua escolha) e restaurar em outra máquina sem precisar baixar tudo de novo do provedor. O backup deve ser legível só por quem tem a chave, e o formato deve ser documentado, para que você nunca fique preso ao Epistula.

### Gestão de e-mail de verdade

Busca local no conteúdo completo, mesmo offline. Regras que também rodam no servidor, com o app fechado. Adiar mensagens. Visão unificada de todas as contas. Assinaturas, modelos e agendamento de envio. Outros provedores (Outlook, Yahoo, iCloud e IMAP genérico). Indicadores de autenticidade do remetente (SPF, DKIM, DMARC) visíveis na leitura, para ajudar a reconhecer phishing.

## Como a sua informação é tratada

```
Seu computador
├── Interface (WebView)      só exibe; nunca vê tokens nem a chave do banco
├── Processo Rust            OAuth, IMAP, SMTP, limpeza de HTML, decide o que sai para a tela
├── Cofre do sistema         refresh tokens e a chave do banco
└── epistula.db              mensagens, pastas e ajustes, cifrado por inteiro

        │ TLS
        ▼
Seu provedor de e-mail
```

Alguns pontos que valem ser ditos com clareza:

- **O seu provedor continua lendo os seus e-mails**, a menos que eles vão criptografados com OpenPGP. O Epistula protege a cópia local e o caminho até ela; o que sai sem criptografia continua legível por quem hospeda a caixa, e mesmo o que vai criptografado mostra ao servidor quem escreveu para quem e quando.
- **Os logs não contêm conteúdo.** Nem assunto, nem remetente, nem endereço da conta. Um teste percorre o app inteiro com marcadores únicos e falha se algum aparecer no log ou em texto puro no arquivo do banco.
- **Sair de uma conta revoga o acesso** no provedor e apaga o segredo do cofre do sistema. As mensagens já baixadas continuam no disco, cifradas, até que exista a opção de apagá-las.
- **Um computador comprometido continua comprometido.** Malware rodando na sua sessão consegue ler o que você consegue ler. Nenhum cliente de e-mail resolve isso, e preferimos dizer isso a fingir o contrário.

A análise completa, com a matriz de dados, o inventário de criptografia, os limites de confiança e as dezessete ameaças consideradas, está em [docs/arquitetura-e-ameacas.md](docs/arquitetura-e-ameacas.md).

## Design System

Um cliente de e-mail é aberto dezenas de vezes por dia, e qualquer atrito pequeno vira irritação grande. Por isso a interface nasce de um Design System próprio, em [`design-system/`](design-system/), com tokens, tipografia, cores, acessibilidade e uma página para cada componente. Ele está em evolução constante, e toda tela nova passa por ele antes de virar código.

Quatro princípios orientam as decisões: densidade com respiro, leitura em primeiro lugar, ação a um gesto e estado sempre visível. Sugestões de interface e usabilidade são tão bem-vindas quanto código.

## Compilando

O alvo principal hoje é o Windows. O Epistula é feito com [Tauri 2](https://tauri.app), Rust e React.

Requisitos:

- Node.js 24 e pnpm 10
- Rust 1.88 ou mais recente
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
src-tauri/src/       backend em Rust (OAuth, IMAP, SMTP, sincronização, banco, limpeza de HTML, notificações)
src-tauri/migrations versões do esquema do banco
design-system/       componentes e tokens visuais
docs/                arquitetura e modelo de ameaças
userstory/           histórico de decisões, uma story por entrega
```

## Contribuindo

Issues e pull requests são bem-vindos, em português ou em inglês. Antes de abrir um PR que toque em credenciais, armazenamento ou exibição de conteúdo de terceiros, leia o modelo de ameaças: mudanças nessas áreas precisam dizer qual ameaça afetam e como foram testadas.

Encontrou uma falha de segurança? Não abra uma issue pública. Escreva para os mantenedores pelo GitHub em privado e daremos retorno antes de qualquer divulgação.

## Licença

A definir.

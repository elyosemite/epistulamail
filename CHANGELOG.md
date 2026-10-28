# Mudanças

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e as versões seguem o [SemVer](https://semver.org/lang/pt-BR/). O pipeline de release (`.github/workflows/release.yml`) usa a seção da versão como notas da release.

## [Não lançado]

### Novidades

- Versão para Linux: AppImage e pacote `.deb`, com as mensagens no mesmo banco cifrado e as chaves no Secret Service (GNOME Keyring ou KWallet).
- O site ganhou um botão de download com menu para Windows e Linux.
- No Linux, o app usa o espaço livre do disco, as raízes de confiança do sistema e o aviso de disco sem cifra (LUKS).

### Mudou

- Os textos do app, do site e da documentação falam em "sistema" em vez de "Windows" onde o comportamento vale para os dois.
- "Bloquear junto com o Windows" só aparece no Windows; no Linux, o bloqueio por inatividade continua.

### Avisos

- No Linux, o app precisa do Secret Service para abrir. Sem ele, avisa e não altera os dados.
- As notificações nativas no Linux e o bloqueio junto com a tela chegam numa próxima versão (US18).
- O AppImage se atualiza sozinho; o `.deb` é atualizado pelo gerenciador de pacotes.
- Os pacotes Linux ainda não têm assinatura de código: confira o SHA-256 nas notas da versão.

## [1.0.2] - 2026-10-27

### Novidades

- Atualização automática: quando sai uma versão nova, aparece um aviso no sino. Um clique mostra o que mudou e, se você confirmar, o EpistulaMail baixa a versão nova, confere a assinatura dela, instala e abre de novo. As contas e as mensagens continuam.

### Mudou

- O app passa a se chamar EpistulaMail: na janela, na bandeja, nos avisos, no menu Iniciar, no instalador (`EpistulaMail_<versão>_x64-setup.exe`) e na lista de programas do Windows. A instalação anterior ("Epistula") é substituída sem perder dados.
- O instalador usa o ícone do EpistulaMail.
- A tela de login não cita mais o nome do provedor.

### Corrigido

- Espaçamento e alinhamento dos botões e campos nas telas de criptografia, chaves, certificados e regras.

### Avisos

- A atualização automática vale a partir desta versão: para chegar a ela, instale a 1.0.2 por cima uma vez. Daqui em diante, as próximas chegam pelo sino.
- O instalador continua sem assinatura de código: confira o SHA-256 abaixo antes de executar.

## [1.0.1] - 2026-10-27

### Mudou

- Novo ícone do EpistulaMail no app, no instalador, na barra de tarefas, no menu Iniciar e na bandeja do sistema, sem fundo e legível de 16 a 256 px.

### Avisos

- O instalador continua sem assinatura de código: confira o SHA-256 abaixo antes de executar. Instale por cima da 1.0.0; as contas e as mensagens continuam.

## [1.0.0] - 2026-10-27

Primeira versão publicada, para Windows 10 e 11 (64 bits).

### Contas e segurança

- Contas Google por OAuth (PKCE), com leitura por IMAP e envio por SMTP, sempre com TLS. A senha da conta nunca passa pelo Epistula.
- Várias contas ao mesmo tempo, cada uma com nome, cor e foto.
- Banco local SQLCipher (AES-256, HMAC por página), com a chave no Gerenciador de Credenciais do Windows e apagamento seguro das páginas liberadas.
- Senha do Epistula opcional (Argon2id), bloqueio automático por inatividade ou com o Windows, e "guardar menos": apagar do computador o que está fora do período escolhido e as contas de que você saiu.
- Leitura segura: o HTML das mensagens é limpo no processo Rust e mostrado sem scripts; imagens externas ficam bloqueadas até você liberar. Aviso quando um link leva a outro lugar do que mostra.
- Notificações sem conteúdo por padrão.

### Leitura

- Conversas em cartões, com o texto citado recolhido.
- Leitura sem conexão: as mensagens do período escolhido são baixadas em segundo plano, sem marcar como lidas.
- Abertura instantânea: as próximas mensagens da lista e o resto da conversa são buscados antes.

### Escrever e organizar

- Escrever, responder, responder a todos e encaminhar, com rascunhos cifrados e "Desfazer envio".
- Pastas e marcadores; sinalizar com prazo; mover, arquivar, excluir e marcar como lida em lote, também arrastando para uma pasta.
- Barra de comandos, densidade da lista, colunas recolhíveis, tema claro, escuro ou do sistema e cor de fundo.

### Busca

- Em todas as pastas, sem internet, numa conta ou em todas (arquivos de e-mail incluídos).
- Pessoas, período e filtros como chips no campo, também pelos operadores `de:`, `para:`, `cc:`, `depois:`, `antes:`, `em:`, `tem:anexo`, `é:não-lida`, `é:estrela` e `na:`.
- Sugestões com as pessoas com quem você já trocou e-mails, buscas recentes neste computador e buscas salvas na coluna de pastas, com as não lidas.
- Ações e regras aplicadas aos resultados, com prévia e "Desfazer".

### Regras

- Regras que organizam os e-mails assim que chegam, lidas como frases, com condições, exceções e várias ações.
- Prévia antes de salvar, histórico de 30 dias com "Desfazer", modelos prontos e criação a partir de uma mensagem.
- "Rodar agora" numa pasta e regras de limpeza diárias.

### Criptografia de ponta a ponta

- OpenPGP: criar chave (Curve25519, RSA 4096 ou pós-quântica experimental ML-KEM-768 + X25519), importar, exportar e fazer backup com senha. Autocrypt e chaves anexadas entram como "ainda não aceitas".
- S/MIME: pedir certificado (o par de chaves é criado no computador), importar `.p12` e guardar os certificados de quem envia mensagens assinadas, com a confiança das raízes do Windows.
- O cadeado do composer liga sozinho quando todos os destinatários têm chave ou certificado; o assunto pode ir protegido.

### Arquivos de e-mail, sem conta

- Abrir backups e pastas de outros programas: `.mbox` do Google Takeout, `.pst` e `.msg` do Outlook, pastas do Thunderbird, `.mbox` e `.emlx` do Apple Mail, `.eml` e Maildir.
- Sem conta, sem servidor e sem rede: o arquivo é lido no computador e copiado para o banco cifrado; o original só é lido.

### Avisos

- **Instalador sem assinatura de código.** Na primeira execução, o Windows SmartScreen mostra "O Windows protegeu o computador". Confira o SHA-256 publicado na release, clique em "Mais informações" e depois em "Executar assim mesmo".
- Só contas Google por enquanto. Outlook, Yahoo, iCloud e IMAP genérico ficam para as próximas versões.
- O instalador precisa do WebView2, que já vem no Windows 11; no Windows 10 sem ele, o instalador baixa da Microsoft.
- Ainda não há atualização automática: as novas versões são instaladas por cima, mantendo as contas e as mensagens.

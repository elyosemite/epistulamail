## User Story 15: Abrir arquivos de e-mail de outros programas, sem conta

**Como** usuário do Epistula,
**quero** abrir no Epistula um backup ou arquivo de e-mail do Gmail, do Outlook, do Thunderbird, do Apple Mail ou de outro programa,
**para que** eu leia, pesquise e guarde meus e-mails antigos com a mesma proteção do resto, sem precisar de conta nem de ligação com servidor nenhum.

---

### 1. Contexto

Pedido em 26 out 2026, como a próxima entrega. Regra do produto: a importação funciona **sem nenhuma conta conectada e sem nenhuma ligação com o servidor do Epistula** (o servidor de login planejado, ver `docs/arquitetura-e-ameacas.md`, "Planejado: servidor de login") **nem com a internet**.

Hoje, sem conta, o app mostra só a tela de conectar (`App.tsx`, `accounts.length === 0`). As mensagens vivem no `epistula.db` presas a uma conta (`account`, `folder`, `message`), com o corpo RFC 822 guardado e lido pelo `mail_parser`; a leitura segura (limpeza de HTML, imagens bloqueadas), as conversas, a busca, as regras e a criptografia (US13, US14) já trabalham sobre esse corpo.

### 2. Formatos

| Origem | O que o usuário tem | Formato | Observações |
|---|---|---|---|
| Gmail | Google Takeout | `.mbox` único (um arquivo com todos os e-mails) | Cabeçalho `X-Gmail-Labels` com os marcadores; `X-GM-THRID` com a conversa |
| Outlook (Windows) | Arquivo → Exportar | `.pst` | Binário (MS-PST); pastas, mensagens, anexos; pode ter senha (fraca, só no cabeçalho) |
| Outlook (Windows) | Cache da conta | `.ost` | Mesmo formato do `.pst`; D3 |
| Outlook | Mensagem arrastada | `.msg` | OLE/CFB (MS-OXMSG) |
| Outlook para Mac | Exportar | `.olm` | ZIP com XML e anexos |
| Thunderbird | Pasta do perfil (`Mail/`, `ImapMail/`) | mbox sem extensão, com `.msf` ao lado | O `.msf` é índice e é ignorado; mensagens apagadas mas não compactadas têm `X-Mozilla-Status` |
| Apple Mail | Caixa → Exportar | Pacote `.mbox` (pasta com o arquivo `mbox`) | |
| Apple Mail | Pasta da biblioteca | `.emlx` (tamanho + RFC 822 + plist) | |
| Qualquer um | | `.eml`, mbox, Maildir (`cur/`, `new/`) | |

### 3. Critérios de aceite

**CA1. Abrir sem conta**

- A primeira tela (sem conta) oferece "Abrir arquivo de e-mail" ao lado de conectar uma conta. Com contas, a opção fica no menu da conta e em Configurações.
- Escolher um arquivo ou uma pasta (também arrastar para a janela). O app reconhece o formato pelo conteúdo, não só pela extensão.
- Nenhuma requisição de rede durante a importação nem ao abrir o que foi importado (as imagens externas continuam bloqueadas até o usuário liberar, como em qualquer mensagem).

**CA2. Importar**

- Antes de importar: o formato reconhecido, o tamanho, as pastas encontradas (marcar quais entram) e a estimativa de espaço no disco.
- Progresso com quantidade e "Cancelar"; o que já entrou fica. Fechar o app no meio retoma ou descarta, sem deixar o banco pela metade (D6).
- No fim: quantas entraram, quantas eram repetidas, quantas falharam e por quê (sem mostrar conteúdo no log).
- O arquivo original só é lido, nunca alterado nem apagado. O app avisa que o original continua no disco sem cifra.

**CA3. Ler e organizar**

- O que foi importado aparece como um **arquivo local** na barra de contas (nome do arquivo, editável), com as pastas de origem; os marcadores do Takeout viram marcadores (D5).
- Leitura, conversas, busca, mover entre as pastas do arquivo, sinalizar e excluir funcionam como numa conta; não há envio nem sincronização.
- Mensagens OpenPGP e S/MIME do arquivo abrem com as chaves e certificados que já existem; o que chegar nelas segue as regras da US13 e da US14.
- Remover o arquivo local apaga as mensagens dele do banco e troca a chave do banco (como sair de uma conta, US10).

**CA4. Segurança**

- Os leitores rodam no processo Rust, com limites por mensagem e por anexo, limite de descompressão no `.olm` e nada extraído para fora do banco (sem arquivos temporários).
- Um arquivo malformado falha naquela mensagem ou pasta, nunca derruba o app nem corrompe o banco.
- Logs: só contagens e o tipo do erro; nunca o caminho do arquivo, nomes de pastas, assuntos ou endereços.

### 4. Backend (a avaliar)

- mbox, Maildir, `.eml` e `.emlx`: leitura própria em streaming (o Takeout passa fácil de dezenas de GB), com o `mail_parser` para cada mensagem. Variantes do mbox (`From ` escapado como `>From `, mboxrd/mboxo, `Content-Length`).
- `.pst`/`.ost`: avaliar o crate `outlook-pst` (publicado pela Microsoft) e alternativas; a mensagem do PST não é RFC 822 e precisa ser remontada (propriedades MAPI → MIME), inclusive o corpo em RTF compactado.
- `.msg`: `cfb` + as mesmas propriedades MAPI do PST.
- `.olm`: `zip` + XML (`quick-xml`).
- Banco: uma "conta" de tipo arquivo local (`provider = 'archive'`, sem token nem servidor) ou tabela própria (D1); repetidas detectadas por `Message-ID` e hash do corpo.

### 5. Decisões

- **D1 (pendente, recomendo dentro do banco):** onde fica o que foi importado. **A.** Copiado para o `epistula.db`, cifrado, como uma conta do tipo arquivo local (recomendado: uma só proteção, busca e leitura sem conexão iguais às das contas; custa espaço no disco). **B.** Lido do arquivo original a cada vez, guardando só um índice (sem duplicar espaço, mas o conteúdo fica fora do banco cifrado e some se o arquivo mudar de lugar).
- **D2 (pendente, recomendo não nesta story):** mandar mensagens importadas para uma conta conectada (IMAP `APPEND`). Útil para migrar de provedor, mas é envio de dados para um servidor; fica para outra story.
- **D3 (pendente, recomendo só se o leitor aguentar):** `.ost`. É o cache do Outlook, muitas vezes do Exchange; o `.pst` exportado é o caminho garantido. Recomendo aceitar o `.ost` como "tentativa", com aviso.
- **D4 (pendente):** quais formatos entram nesta entrega. Recomendo: mbox (Takeout, Thunderbird, Apple Mail), `.eml`, `.emlx`, Maildir e `.pst`/`.msg`; o `.olm` em seguida, se o `.pst` tomar a entrega inteira.
- **D5 (pendente, recomendo marcadores):** no Takeout, cada mensagem tem vários marcadores (`X-Gmail-Labels`). Recomendo: "Caixa de entrada", "Enviados" e afins viram pastas; o resto, marcadores, como numa conta do Google.
- **D6 (pendente, recomendo retomar):** importação interrompida (app fechado, falta de espaço). Recomendo retomar de onde parou, já que as repetidas são detectadas; alternativa é descartar o que entrou daquele arquivo.
- **D7 (pendente):** limite de espaço. Avisar antes quando o disco não comporta e parar com margem (o C: cheio já foi um problema neste projeto).
- **D8 (pendente, recomendo não):** rodar as regras (US12) nas mensagens importadas. Recomendo não, para o arquivo ficar como estava; "Executar agora" continua possível.

### 6. Fora do escopo

- Importar contatos e calendários (`.vcf`, `.ics`, o resto do Takeout).
- Exportar para esses formatos (o "Backup que você controla" do README é outra story).
- Enviar o que foi importado para um servidor (D2).

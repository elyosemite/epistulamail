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
| Outlook (Windows) | Cache da conta | `.ost` | Mesmo formato do `.pst`; D2 |
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
- Progresso com quantidade e "Cancelar"; o que já entrou fica. Fechar o app no meio retoma ou descarta, sem deixar o banco pela metade (D5). Sem espaço no disco, avisa antes e não começa (D6).
- No fim: quantas entraram, quantas eram repetidas, quantas falharam e por quê (sem mostrar conteúdo no log).
- O arquivo original só é lido, nunca alterado nem apagado. O app avisa que o original continua no disco sem cifra.

**CA3. Ler e organizar**

- O que foi importado aparece como um **arquivo local** na barra de contas (nome do arquivo, editável), com as pastas de origem; os marcadores do Takeout viram marcadores (D4).
- Leitura, conversas, busca, mover entre as pastas do arquivo, sinalizar e excluir funcionam como numa conta; não há envio nem sincronização.
- Mensagens OpenPGP e S/MIME do arquivo abrem com as chaves e certificados que já existem; o que chegar nelas segue as regras da US13 e da US14.
- Remover o arquivo local apaga as mensagens dele do banco e troca a chave do banco (como sair de uma conta, US10).

**CA4. Segurança**

- Os leitores rodam no processo Rust, com limites por mensagem e por anexo, limite de descompressão no `.olm` e nada extraído para fora do banco (sem arquivos temporários).
- Um arquivo malformado falha naquela mensagem ou pasta, nunca derruba o app nem corrompe o banco.
- Logs: só contagens e o tipo do erro; nunca o caminho do arquivo, nomes de pastas, assuntos ou endereços.

### 4. Interface

Aprovada em 26 out 2026 e incorporada ao DS em `design-system/arquivos.html` (o rascunho foi apagado): primeira tela com "Abrir arquivo de e-mail", assistente de três passos (escolher, revisar, importar), resumo, o arquivo local na coluna de contas, menu, remover e Configurações → Geral. Peças novas: **L70** área de soltar (`.drop-zone`), **L71** avatar do arquivo local (`.avatar--archive`) e **L72** resumo da importação (`.import-summary`). A confirmar: os nomes de programas ("Google Takeout", "Outlook") aparecem na ajuda de onde achar o backup e no formato reconhecido, frente à regra de não citar o provedor na interface.

### 5. Backend (a avaliar)

- mbox, Maildir, `.eml` e `.emlx`: leitura própria em streaming (o Takeout passa fácil de dezenas de GB), com o `mail_parser` para cada mensagem. Variantes do mbox (`From ` escapado como `>From `, mboxrd/mboxo, `Content-Length`).
- `.pst`/`.ost`: avaliar o crate `outlook-pst` (publicado pela Microsoft) e alternativas; a mensagem do PST não é RFC 822 e precisa ser remontada (propriedades MAPI → MIME), inclusive o corpo em RTF compactado.
- `.msg`: `cfb` + as mesmas propriedades MAPI do PST.
- `.olm`: `zip` + XML (`quick-xml`).
- Banco: uma "conta" de tipo arquivo local (`provider = 'archive'`, sem token nem servidor) ou tabela própria (D1); repetidas detectadas por `Message-ID` e hash do corpo.

### 6. Decisões

- **D1 (pendente, recomendo dentro do banco):** onde fica o que foi importado. **A.** Copiado para o `epistula.db`, cifrado, como uma conta do tipo arquivo local (recomendado: uma só proteção, busca e leitura sem conexão iguais às das contas; custa espaço no disco). **B.** Lido do arquivo original a cada vez, guardando só um índice (sem duplicar espaço, mas o conteúdo fica fora do banco cifrado e some se o arquivo mudar de lugar).
- **D2 (pendente, recomendo só se o leitor aguentar):** `.ost`. É o cache do Outlook, muitas vezes do Exchange; o `.pst` exportado é o caminho garantido. Recomendo aceitar o `.ost` como "tentativa", com aviso.
- **D3 (pendente):** quais formatos entram nesta entrega. Recomendo: mbox (Takeout, Thunderbird, Apple Mail), `.eml`, `.emlx`, Maildir e `.pst`/`.msg`; o `.olm` em seguida, se o `.pst` tomar a entrega inteira.
- **D4 (pendente, recomendo marcadores):** no Takeout, cada mensagem tem vários marcadores (`X-Gmail-Labels`). Recomendo: "Caixa de entrada", "Enviados" e afins viram pastas; o resto, marcadores, como numa conta do Google.
- **D5 (pendente, recomendo retomar):** importação interrompida (app fechado, falta de espaço). Recomendo retomar de onde parou, já que as repetidas são detectadas; alternativa é descartar o que entrou daquele arquivo.
- **D6 (pendente):** limite de espaço. Avisar antes quando o disco não comporta e parar com margem (o C: cheio já foi um problema neste projeto).
- **D7 (pendente, recomendo não):** rodar as regras (US12) nas mensagens importadas. Recomendo não, para o arquivo ficar como estava; "Executar agora" continua possível.

### 7. Implementação (26 out 2026)

Decisões implementadas como as recomendações: **D1** dentro do banco (conta do tipo arquivo local), **D2** `.ost` como tentativa (o mesmo leitor do `.pst`), **D3** todos os formatos menos o `.olm`, **D4** marcadores do Takeout como no Google (pastas com o papel de marcador), **D5** retomar, **D6** parar sem espaço com margem de 1 GB, **D7** as regras não rodam.

- **Backend:**
  - `mail_import.rs`: reconhece pelo conteúdo (`From ` do mbox, `X-Gmail-Labels` do Takeout, `!BDN` do `.pst`, OLE do `.msg`, tamanho na primeira linha do `.emlx`, cabeçalhos do `.eml`); acha as pastas (Thunderbird com `.sbd` e `.msf`, pacotes `.mbox` do Apple Mail com `mbox` exportado ou `.emlx` da biblioteca, Maildir e Maildir++, pastas com `.eml` ou `.msg`) e lê em streaming. O mbox só separa num `From ` depois de linha em branco, tira um `>` do `>From `, respeita `X-Mozilla-Status` (apagada e não compactada não entra), `Status` e `X-Status`; flags do Maildir (`:2,` ou `!2,` no Windows) e do plist do `.emlx`. Mensagem acima de 50 MB falha sozinha.
  - `mapi_import.rs`: `.pst` pelo `outlook-pst` (Microsoft, MIT, só leitura) e `.msg` pelo `cfb`. As propriedades MAPI viram MIME: os cabeçalhos originais da internet (`PR_TRANSPORT_MESSAGE_HEADERS`) quando existem, senão De, Para, Cc, Assunto, Data, Message-ID, In-Reply-To e References remontados; texto, HTML (com o charset do code page) e anexos, inclusive mensagem anexada (`message/rfc822`). Só `IPM.Note` (calendário, contatos, tarefas e notas ficam de fora); pastas `IPF.Note`. A "senha" do `.pst` é só do Outlook e não cifra nada: o arquivo abre sem ela.
  - `archive_store.rs` e a migração 0011 (`archive`): o arquivo local é uma linha de `account` com provider `archive` e status `signed_out` no banco, tratada no Rust como `AccountStatus::Local`. Assim, sincronização, download, notificações, regras e o índice do cofre nunca o tocam. As mensagens vão para as mesmas tabelas de uma conta (corpo, anexos, prévia, conversa): lista, leitura, conversas, busca e criptografia funcionam iguais. Repetida é a mensagem com o mesmo Message-ID e tamanho (sem Message-ID, o SHA-256 do conteúdo) que já está em todas as pastas pedidas. No fim, os UIDs de cada pasta seguem a data (a lista é ordenada pelo UID).
  - `import.rs`: exame, criação, leitor numa thread que manda as mensagens já interpretadas para lotes de 200 (ou 32 MB) gravados numa transação, progresso por evento, "Cancelar", espaço livre conferido antes (`GetDiskFreeSpaceExW`) e a cada lote, e retomada: na abertura, o que estava importando vira "interrompida". O seletor nativo (`tauri-plugin-dialog`) é aberto pelo Rust; a interface não recebe permissão de diálogo.
  - Num arquivo local, ler marca como lida só no banco; mover, arquivar ("Arquivados" criada na primeira vez), excluir para a Lixeira (com "Desfazer"), excluir de vez, estrela, sinalização e criar pasta acontecem só no banco. Remover apaga a conta inteira e troca a chave do banco (como apagar os dados de uma conta, US10).
- **Frontend:** primeira tela com "Abrir arquivo de e-mail" sob "Sem conta"; assistente de três passos e resumo (`features/import/ImportDialog.tsx`); arrastar sobre a janela mostra a área de soltar (L70), com o `dragDropEnabled` da janela ligado; o arquivo local na coluna de contas (L71), "Arquivo deste computador" e o progresso na coluna de pastas; barra de comandos sem escrever, responder e regras; menu do arquivo (renomear, importar mais, abrir outro, remover); Configurações → Geral → Arquivos de e-mail; aviso de importação interrompida com "Continuar" e "Descartar o que entrou". Sincronização, notificações, regras e criptografia não listam arquivos locais.
- **DS:** `arquivos.html` (L70 área de soltar, L71 avatar do arquivo local, L72 resumo) e `.modal--medium` (assistente com listas). Com o arrastar nativo ligado, a ordem das regras (US12) passou a ser arrastada com o ponteiro pela alça, porque o arraste do HTML5 não funciona com ele no Windows.
- **Limitações desta entrega:**
  - `.olm` (Outlook para Mac) não abre (D3).
  - `.ost` não foi testado com um arquivo de verdade: é tentado pelo mesmo leitor do `.pst`.
  - No `.pst` e no `.msg`, uma mensagem que só tem o corpo em RTF (sem texto nem HTML) aparece sem corpo; S/MIME guardado pelo Outlook (`IPM.Note.SMIME`) vira uma mensagem com o anexo `smime.p7m`, que a leitura não decifra; a mensagem anexada a outra no `.pst` vem sem os anexos dela; endereços só do Exchange (`/O=…`) aparecem só com o nome.
  - Responder e encaminhar não aparecem num arquivo local (o rascunho previa abrir o composer de uma conta conectada).
  - Enquanto importa, a lista segue a ordem do arquivo; a ordem pela data vem no fim.
  - Bloquear o Epistula durante uma importação a interrompe (o banco fecha); o aviso de continuar aparece depois de desbloquear.
  - O espaço livre só é conferido no Windows.
- **Testes:** 205 no Rust (formatos, Takeout com marcadores, Thunderbird com subpastas e apagadas, Maildir, Apple Mail, `.emlx`, `.msg` montado no teste, `.pst` vazio do crate, MAPI → MIME com HTML em windows-1252 e anexos, repetidas, ordem por data, mover e excluir sem servidor, interrompida, do arquivo ao banco) e 101 no frontend; clippy, typecheck e build sem avisos.
- **Falta (manual):** abrir no app um Takeout real, um `.pst` do Outlook, um perfil do Thunderbird e um `.mbox` do Apple Mail; conferir o seletor nativo e o arrastar no WebView2.

### 8. Fora do escopo

- Importar contatos e calendários (`.vcf`, `.ics`, o resto do Takeout).
- Exportar para esses formatos (o "Backup que você controla" do README é outra story).
- Enviar o que foi importado para um servidor (IMAP `APPEND`), por exemplo para migrar de provedor.

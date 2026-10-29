/* Epistula Design System · fichas.js
   A ficha de cada componente (v3 §7): onde é usado, fazer e não fazer, texto e tradução, testes, plataforma,
   relacionados e histórico. O docs.js monta a ficha no fim da página; a camada, o status e os tokens vêm de lá.
   Para um componente novo, acrescente uma entrada com o mesmo id da página. Texto sem classes CSS. */
const EPISTULA_FICHAS = {
  "botoes": {
    "usadoEm": [
      "src/features/accounts/AccountDialogs.tsx",
      "src/features/accounts/AddAccount.tsx",
      "src/ui/Button.tsx",
      "src/ui/CommandBar.tsx",
      "src/ui/InlineCompose.tsx",
      "src/ui/PasswordField.tsx"
    ],
    "fazer": [
      "Um botão primário por área: a ação que conclui o fluxo.",
      "Rótulo com verbo que diz o resultado: \"Arquivar\", \"Enviar\".",
      "Estilo sutil para barras de ferramentas e ações em linha."
    ],
    "nao": [
      "Não use \"OK\" nem \"Confirmar\" como rótulo.",
      "Não use o botão de perigo para o que tem Desfazer: excluir uma conversa é sutil, com toast.",
      "Não dependa só da cor para o estado: desabilitado também perde o contraste e o cursor."
    ],
    "conteudo": "O botão cresce com o texto: nunca fixe a largura. Rótulos curtos, no infinitivo; sem juntar pedaços de frase. Botão só com ícone leva <code>aria-label</code> e dica.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Usado em todo o app; o comportamento de cada ação é testado na lógica de cada tela. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica: igual no Windows e no Linux. O botão dividido de download escolhe o pacote pelo sistema detectado (L81).",
    "relacionados": [
      "campos",
      "menus",
      "modais",
      "padrao-confirmacao"
    ],
    "historico": [
      [
        "L81",
        "Botão dividido de download, com Windows e Linux (US17)."
      ]
    ]
  },
  "campos": {
    "usadoEm": [
      "src/features/accounts/AccountDialogs.tsx",
      "src/features/crypto/AddKeyDialog.tsx",
      "src/features/crypto/SmimeDialogs.tsx",
      "src/features/import/ImportDialog.tsx",
      "src/features/lock/UnlockScreen.tsx",
      "src/features/settings/OfflineSettings.tsx",
      "src/features/settings/SyncSettings.tsx",
      "src/layout/NotificationBell.tsx",
      "src/ui/Field.tsx",
      "src/ui/PasswordField.tsx",
      "src/ui/Select.tsx"
    ],
    "fazer": [
      "Valide ao sair do campo, nunca a cada tecla.",
      "Diga o que corrigir, não só o que deu errado.",
      "Associe rótulo, ajuda e erro ao campo (<code>for</code>, <code>aria-describedby</code>)."
    ],
    "nao": [
      "Não use o placeholder como rótulo.",
      "Não mostre o erro só em vermelho: leva ícone e texto.",
      "Não desligue o gerenciador de senhas: use <code>current-password</code> e <code>new-password</code>."
    ],
    "conteudo": "Mensagens de erro em frases completas e específicas, sem códigos. Evite montar o texto por pedaços ('Campo ' + nome + ' inválido'), porque isso quebra na tradução. Endereços de e-mail e números não têm formato fixo entre idiomas.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica: o campo é o nativo do navegador do sistema.",
    "relacionados": [
      "padrao-validacao",
      "selecao",
      "destinatarios",
      "busca"
    ],
    "historico": []
  },
  "selecao": {
    "usadoEm": [
      "src/features/accounts/AccountDialogs.tsx",
      "src/features/accounts/AddAccount.tsx",
      "src/features/settings/AppearanceSettings.tsx",
      "src/ui/CanvasPicker.tsx",
      "src/ui/ColorPicker.tsx",
      "src/ui/Field.tsx",
      "src/ui/ProviderGrid.tsx",
      "src/ui/Segmented.tsx",
      "src/ui/ThemePicker.tsx"
    ],
    "fazer": [
      "Checkbox para escolher itens; toggle só para ligar e desligar algo com efeito imediato.",
      "No seletor de cor, mostre a cor, a inicial e o nome juntos.",
      "Agrupe opções relacionadas num <code>fieldset</code> com <code>legend</code>."
    ],
    "nao": [
      "Não use toggle para algo que só vale depois de salvar.",
      "Não use só a cor para dizer qual está escolhido: há anel e marca de verificação.",
      "Não peça confirmação para um toggle: o efeito é imediato e reversível."
    ],
    "conteudo": "O texto do toggle descreve o estado ligado ('Bloquear junto com o sistema'). Nomes de cor são traduzíveis e também são o nome acessível.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A escolha de aparência é coberta em <code>src/lib/us6.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "O toggle 'Bloquear junto com o Windows' só existe no Windows; no Linux o bloqueio por inatividade continua.",
    "relacionados": [
      "campos",
      "temas",
      "avatar",
      "modais"
    ],
    "historico": [
      [
        "L17",
        "Seletor de cor da conta."
      ]
    ]
  },
  "avatar": {
    "usadoEm": [
      "src/features/settings/SyncSettings.tsx",
      "src/layout/AccountsBarItem.tsx",
      "src/ui/AccountAvatar.tsx",
      "src/ui/Avatar.tsx"
    ],
    "fazer": [
      "Mostre a inicial na cor da conta e a foto por cima, quando existir.",
      "Use o tamanho do contexto: 20px em chips, 24px em listas densas, 32px padrão, 40px na coluna de contas.",
      "Deixe o nome da pessoa ou da conta no texto ao lado."
    ],
    "nao": [
      "Não use o avatar para contar ou dar status: isso é do badge e do lozenge.",
      "Não carregue foto de serviço externo no app: a foto chega pelo backend.",
      "Não use foto como única pista de quem é a conta."
    ],
    "conteudo": "A inicial vem do nome que a pessoa deu à conta, em qualquer alfabeto. O avatar é decorativo para leitor de tela; o nome fica em texto.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "badge",
      "lozenge",
      "contas",
      "destinatarios"
    ],
    "historico": [
      [
        "L30",
        "Foto sobre a inicial."
      ]
    ]
  },
  "badge": {
    "usadoEm": [
      "src/features/mailbox/Mailbox.tsx",
      "src/features/message/MessageView.tsx",
      "src/layout/AccountsBar.tsx",
      "src/layout/AccountsBarItem.tsx",
      "src/layout/EmailItem.tsx",
      "src/layout/EmailList.tsx",
      "src/ui/Conversation.tsx",
      "src/ui/NoticePanel.tsx",
      "src/ui/Thread.tsx"
    ],
    "fazer": [
      "Use só para número: não lidas, itens num lote.",
      "Mostre '99+' acima de 99.",
      "Diga no item o que está sendo contado ('Caixa de entrada, 12 não lidas')."
    ],
    "nao": [
      "Não escreva palavras num badge: use o lozenge.",
      "Não use o número como única pista de novidade: o item também vai em negrito.",
      "Não coloque badge em tudo: só onde a contagem muda a decisão."
    ],
    "conteudo": "Números seguem o formato do idioma (separador de milhar). O texto do badge não é traduzido; o rótulo do item que o contém é.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "lozenge",
      "avatar",
      "navegacao",
      "contas"
    ],
    "historico": []
  },
  "lozenge": {
    "usadoEm": [
      "src/ui/Lozenge.tsx"
    ],
    "fazer": [
      "Use uma palavra curta que diga o status.",
      "Prefira o estilo sutil, em linha com o texto.",
      "Escolha a cor pelo sentido: sucesso, falha, em andamento, novo, agendada."
    ],
    "nao": [
      "Não use número num lozenge: use o badge.",
      "Não troque a palavra por um ponto colorido.",
      "Não use o forte se o status não é o assunto principal da tela."
    ],
    "conteudo": "Uma palavra, no tempo certo ('Enviando', 'Enviada'). Traduza como palavra solta, sem depender de gênero ou plural de outra frase.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "badge",
      "avatar",
      "governanca"
    ],
    "historico": [
      [
        "2026-10-28",
        "Passou a mostrar a maturidade de cada página do DS."
      ]
    ]
  },
  "menus": {
    "usadoEm": [
      "src/features/message/MessageView.tsx",
      "src/layout/AccountMenu.tsx",
      "src/layout/AccountsBar.tsx",
      "src/layout/CommandBar.tsx",
      "src/ui/Lozenge.tsx",
      "src/ui/Menu.tsx",
      "src/ui/Segmented.tsx",
      "src/ui/Thread.tsx",
      "src/ui/TooltipLayer.tsx"
    ],
    "fazer": [
      "Agrupe ações e destinos, com o atalho à direita.",
      "Abra no gesto e devolva o foco a quem abriu ao fechar.",
      "Dê um tooltip a todo botão só com ícone, no hover e no foco."
    ],
    "nao": [
      "Não pinte 'Sair da conta' de vermelho: os dados ficam no computador.",
      "Não use menu para navegar entre seções: use a navegação lateral.",
      "Não esconda uma ação essencial só num menu."
    ],
    "conteudo": "Itens começam com verbo ('Mover para', 'Renomear'). O atalho mostra a tecla do sistema. Reticências quando abre outra janela ('Selecionar outra pasta…').",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A montagem do menu da conta é testada em <code>src/lib/us7.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "O atalho mostra Ctrl no Windows e no Linux.",
    "relacionados": [
      "navegacao",
      "botoes",
      "modais",
      "contas"
    ],
    "historico": []
  },
  "mensagens": {
    "usadoEm": [
      "src/features/accounts/AddAccount.tsx",
      "src/features/import/ImportDialog.tsx",
      "src/features/lock/UnlockScreen.tsx",
      "src/features/mailbox/Mailbox.tsx",
      "src/features/message/ConversationView.tsx",
      "src/features/message/MessageView.tsx",
      "src/features/settings/SyncSettings.tsx",
      "src/features/update/UpdateDialog.tsx",
      "src/ui/Banner.tsx",
      "src/ui/SectionMessage.tsx"
    ],
    "fazer": [
      "Banner para o que é do app inteiro: conexão, conta, manutenção.",
      "Section message para o que é de uma mensagem ou configuração.",
      "Diga o que aconteceu, o efeito provável e a ação disponível."
    ],
    "nao": [
      "Não prometa segurança absoluta.",
      "Não use alarme para o que não precisa de ação.",
      "Não dependa só da cor: há ícone e título."
    ],
    "conteudo": "Frases completas, calmas e concretas. Evite jargão. O botão de ação repete o verbo ('Reconectar').",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "toasts",
      "modais",
      "padrao-validacao",
      "vazios"
    ],
    "historico": []
  },
  "toasts": {
    "usadoEm": [
      "src/features/mailbox/Mailbox.tsx",
      "src/ui/Toast.tsx"
    ],
    "fazer": [
      "Confirme a ação que acabou de acontecer e ofereça 'Desfazer'.",
      "Deixe a mensagem visível por 6 segundos; erros ficam até a pessoa fechar.",
      "Anuncie o aviso em uma região <code>aria-live=\"polite\"</code>."
    ],
    "nao": [
      "Não use toast para uma decisão: use o modal.",
      "Não ponha dados sensíveis no resumo da notificação do sistema.",
      "Não cite o provedor da conta no texto."
    ],
    "conteudo": "Quem ou o quê, depois o assunto e o começo do texto, e por último o nome que a pessoa deu à conta.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A lógica de notificações é testada em <code>src/lib/us8.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "As notificações nativas funcionam no Windows. No Linux elas chegam na US18.",
    "relacionados": [
      "modais",
      "mensagens",
      "padrao-confirmacao"
    ],
    "historico": [
      [
        "US8",
        "Notificações do sistema e da bandeja."
      ]
    ]
  },
  "modais": {
    "usadoEm": [
      "src/App.tsx",
      "src/features/accounts/AccountDialogs.tsx",
      "src/features/crypto/CertManager.tsx",
      "src/features/crypto/KeyManager.tsx",
      "src/features/import/ArchiveSettings.tsx",
      "src/features/import/ImportDialog.tsx",
      "src/features/lock/UnlockScreen.tsx",
      "src/features/mailbox/Mailbox.tsx",
      "src/features/message/ComposeCard.tsx",
      "src/features/message/LinkCheckDialog.tsx",
      "src/features/rules/RuleDialogs.tsx",
      "src/features/rules/RulesSettings.tsx",
      "src/features/settings/AppearanceSettings.tsx",
      "src/features/settings/GeneralSettings.tsx",
      "src/features/settings/NotificationSettings.tsx",
      "src/features/settings/OfflineSettings.tsx",
      "src/features/settings/PrivacyDialogs.tsx",
      "src/features/settings/PrivacySettings.tsx",
      "src/features/settings/Settings.tsx",
      "src/features/settings/SyncSettings.tsx",
      "src/features/update/UpdateDialog.tsx",
      "src/layout/CommandBar.tsx",
      "src/ui/ConfirmCheck.tsx",
      "src/ui/FolderPicker.tsx",
      "src/ui/Modal.tsx",
      "src/ui/SettingsSheet.tsx"
    ],
    "fazer": [
      "Reserve o modal para decisões sem Desfazer.",
      "Título como pergunta ou ação; o botão repete o verbo.",
      "Comece o foco em 'Cancelar', prenda o foco no modal e feche com Esc."
    ],
    "nao": [
      "Não use modal para confirmar o que tem Desfazer.",
      "Não pinte de perigo o que não apaga dados.",
      "Não empilhe modais."
    ],
    "conteudo": "Título curto e botão com o mesmo verbo ('Excluir permanentemente'). Explique a consequência antes da confirmação.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. O foco preso e o Esc estão em <code>src/ui/Modal.tsx</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "padrao-confirmacao",
      "toasts",
      "mensagens",
      "botoes"
    ],
    "historico": []
  },
  "navegacao": {
    "usadoEm": [
      "src/features/accounts/AddAccount.tsx",
      "src/ui/Sidebar.tsx"
    ],
    "fazer": [
      "Use para poucos destinos que a pessoa troca com frequência.",
      "Marque o atual com o fundo de seleção e <code>aria-current</code>.",
      "Dê uma dica a cada item quando a barra estiver recolhida."
    ],
    "nao": [
      "Não ponha ações aqui: ações vão em menu ou botão.",
      "Não dependa só do número para mostrar novidade.",
      "Não leve regra de e-mail para o componente: isso é de Pastas e Contas."
    ],
    "conteudo": "Nomes curtos, no singular ou no plural que faça sentido para o grupo. Evite abreviar: o texto precisa caber traduzido.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "pastas",
      "contas",
      "menus",
      "badge"
    ],
    "historico": [
      [
        "2026-10-28",
        "Separada da página Sidebar, que misturava navegação, pastas e contas (v3 §3)."
      ]
    ]
  },
  "vazios": {
    "usadoEm": [
      "src/features/accounts/AddAccount.tsx",
      "src/features/message/MessageView.tsx",
      "src/features/settings/Settings.tsx",
      "src/layout/EmailListColumn.tsx",
      "src/layout/NotificationBell.tsx",
      "src/ui/EmptyState.tsx"
    ],
    "fazer": [
      "Diga por que está vazio e o que fazer a seguir.",
      "Trate a caixa zerada como uma conquista, não como um erro.",
      "No erro, ofereça uma saída."
    ],
    "nao": [
      "Não mostre uma tela vazia sem texto.",
      "Não use o mesmo estado para 'sem resultado' e 'erro'.",
      "Não culpe a pessoa."
    ],
    "conteudo": "Título curto e uma frase de apoio. A ação principal repete o verbo.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "carregamento",
      "mensagens",
      "busca"
    ],
    "historico": []
  },
  "carregamento": {
    "usadoEm": [
      "src/App.tsx",
      "src/features/import/ImportDialog.tsx",
      "src/features/lock/UnlockScreen.tsx",
      "src/features/message/ConversationView.tsx",
      "src/features/message/MessageView.tsx",
      "src/layout/AppSkeleton.tsx",
      "src/layout/EmailListColumn.tsx",
      "src/ui/InlineCompose.tsx",
      "src/ui/Reader.tsx",
      "src/ui/Spinner.tsx",
      "src/ui/Thread.tsx"
    ],
    "fazer": [
      "Use skeleton quando a forma do conteúdo é conhecida; spinner em ação curta sem forma.",
      "Marque o contêiner com <code>aria-busy=\"true\"</code>.",
      "Respeite 'reduzir movimento': o skeleton para."
    ],
    "nao": [
      "Não use spinner na lista nem no painel de leitura.",
      "Não bloqueie a tela inteira para uma ação local.",
      "Não mostre progresso falso."
    ],
    "conteudo": "Texto de espera só quando há ação ('Sincronizando…'). Tempo de espera depois de um erro mostra a contagem no texto de ajuda.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A espera depois de erros de senha é testada em <code>src/lib/us10.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "vazios",
      "mensagens",
      "toasts"
    ],
    "historico": []
  },
  "contas": {
    "usadoEm": [
      "src/layout/AccountsBar.tsx",
      "src/layout/AccountsBarItem.tsx",
      "src/layout/AppSkeleton.tsx"
    ],
    "fazer": [
      "Uma conta por item, na ordem escolhida pela pessoa.",
      "Reconheça a conta por três sinais: cor, inicial e nome dado.",
      "Mostre o estado: ativa, não lidas, precisa reconectar, saiu."
    ],
    "nao": [
      "Não mostre o e-mail na coluna: ele vai na dica, porque várias contas podem ter o mesmo domínio.",
      "Não pinte 'Sair desta conta' de perigo: os dados ficam.",
      "Não cite o provedor da conta nos textos."
    ],
    "conteudo": "O nome da conta é da pessoa ('Pessoal', 'Trabalho'). O nome acessível junta nome, e-mail e não lidas.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A lógica de contas é testada em <code>src/lib/accounts.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "navegacao",
      "avatar",
      "badge",
      "menus",
      "arquivos"
    ],
    "historico": [
      [
        "L15",
        "Conta na coluna."
      ],
      [
        "2026-10-28",
        "Separada da página Sidebar (v3 §3)."
      ]
    ]
  },
  "pastas": {
    "usadoEm": [
      "src/layout/EmailList.tsx",
      "src/layout/FlaggedItem.tsx",
      "src/layout/FolderItem.tsx",
      "src/layout/FoldersColumn.tsx",
      "src/ui/Sidebar.tsx",
      "src/ui/Thread.tsx"
    ],
    "fazer": [
      "Pastas do sistema no topo, etiquetas abaixo, armazenamento no rodapé.",
      "Em arrastar e soltar, mostre o destino válido e o inválido.",
      "Mantenha o teclado: 'Mover para' continua sendo o caminho sem mouse."
    ],
    "nao": [
      "Não deixe o arrastar como único jeito de mover.",
      "Não mostre 'Sinalizadas' sem mensagens sinalizadas e não concluídas.",
      "Não pinte 'Atrasadas' só de vermelho: o texto diz."
    ],
    "conteudo": "Nomes de pasta do sistema são traduzidos; os das pastas da pessoa não. 'Mover 3 conversas' deve ter plural correto.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. As sinalizadas são testadas em <code>src/lib/us6.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "navegacao",
      "threads",
      "menus",
      "padrao-selecao-em-lote"
    ],
    "historico": [
      [
        "US6",
        "Arrastar para uma pasta e Sinalizadas."
      ]
    ]
  },
  "header": {
    "usadoEm": [
      "src/App.tsx",
      "src/layout/AccountMenu.tsx",
      "src/layout/CommandBar.tsx",
      "src/layout/HeaderBrand.tsx",
      "src/layout/HeaderSearch.tsx",
      "src/layout/NotificationBell.tsx",
      "src/ui/AppBar.tsx",
      "src/ui/CommandBar.tsx",
      "src/ui/FolderPicker.tsx",
      "src/ui/NoticePanel.tsx",
      "src/ui/SearchField.tsx"
    ],
    "fazer": [
      "Busca como elemento central, aberta com <kbd>/</kbd>.",
      "Um botão primário na barra de comandos: 'Novo e-mail'.",
      "Rótulos dos comandos somem abaixo de 1280px; o botão mantém <code>aria-label</code> e dica."
    ],
    "nao": [
      "Não ponha ações da conversa no header: ficam na barra de comandos.",
      "Não esconda a conta ativa: o menu da conta é a saída.",
      "Não use mais de um botão primário."
    ],
    "conteudo": "Comandos com verbo ('Arquivar', 'Mover para'). O atalho aparece na dica.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "menus",
      "botoes",
      "padrao-busca",
      "layouts"
    ],
    "historico": [
      [
        "L76",
        "Logo do EpistulaMail sem fundo."
      ]
    ]
  },
  "threads": {
    "usadoEm": [
      "src/layout/EmailItem.tsx",
      "src/layout/EmailList.tsx",
      "src/layout/EmailListColumn.tsx",
      "src/ui/Thread.tsx",
      "src/ui/Toolbar.tsx"
    ],
    "fazer": [
      "Mostre não lida por três sinais ao mesmo tempo: negrito, ponto e data em azul.",
      "Ofereça as três densidades; a escolha vale para todas as contas.",
      "Deixe todas as ações acessíveis também pelo teclado."
    ],
    "nao": [
      "Não use só a cor para marcar a seleção ou o não lido.",
      "Não abra a conversa ao marcar o checkbox.",
      "Não mostre ações rápidas sem alternativa por teclado."
    ],
    "conteudo": "A data segue o formato do idioma e do dia ('10:42', 'ontem', '23 out'). 'você' nos participantes é traduzido. O trecho corta com reticências; não corte no meio de uma palavra ao traduzir.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A seleção é testada em <code>src/lib/selection.test.ts</code> e as datas em <code>src/lib/format.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "padrao-selecao-em-lote",
      "pastas",
      "leitura",
      "avatar"
    ],
    "historico": [
      [
        "US5",
        "Seleção na lista."
      ],
      [
        "US7",
        "Densidades da lista."
      ]
    ]
  },
  "leitura": {
    "usadoEm": [
      "src/features/message/ConversationView.tsx",
      "src/features/message/MessageView.tsx",
      "src/ui/Conversation.tsx",
      "src/ui/MessageFrame.tsx",
      "src/ui/Reader.tsx"
    ],
    "fazer": [
      "Assunto em Page title e corpo em Body large, com cerca de 70 caracteres por linha.",
      "Deixe as mensagens antigas recolhidas.",
      "Mostre o corpo HTML sempre dentro do quadro isolado."
    ],
    "nao": [
      "Não carregue imagens remotas sem a pessoa permitir.",
      "Não mostre 'Cco' a quem recebeu.",
      "Não rode script do e-mail."
    ],
    "conteudo": "Datas e horas no formato do idioma. Endereços completos nos destinatários. O texto do e-mail é de terceiros e nunca é traduzido pelo app.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A limpeza do HTML tem corpus de ataque e <code>proptest</code> em <code>src-tauri/src/sanitize.rs</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "composicao",
      "anexos",
      "criptografia",
      "threads"
    ],
    "historico": [
      [
        "US2",
        "Pastas e leitura."
      ]
    ]
  },
  "composicao": {
    "usadoEm": [
      "src/features/message/ComposeCard.tsx",
      "src/features/message/MessageView.tsx",
      "src/ui/InlineCompose.tsx"
    ],
    "fazer": [
      "'Enviar' segura a mensagem por 5 segundos, com 'Desfazer'.",
      "Bloqueie o envio com um destinatário inválido e diga o motivo ao lado do botão.",
      "Salve o rascunho no computador."
    ],
    "nao": [
      "Não envie sem a pessoa poder voltar atrás.",
      "Não passe de 25 MB de anexos sem avisar.",
      "Não prometa criptografia que não está ligada."
    ],
    "conteudo": "Prefixos 'Re:' e 'Fw:' são editáveis. A citação segue a convenção do idioma da mensagem original.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. O envio é testado em <code>src/lib/us7.test.ts</code> e a tecnologia de envio em <code>src/lib/us14.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "destinatarios",
      "anexos",
      "criptografia",
      "padrao-confirmacao"
    ],
    "historico": [
      [
        "US7",
        "Envio com desfazer."
      ]
    ]
  },
  "destinatarios": {
    "usadoEm": [
      "src/ui/InlineCompose.tsx"
    ],
    "fazer": [
      "Um chip por endereço, mostrando interno, externo e inválido antes do envio.",
      "Deixe remover com o x e com Backspace.",
      "Diferencie o estado por forma e texto, não só por cor."
    ],
    "nao": [
      "Não aceite um endereço inválido em silêncio.",
      "Não esconda quem é externo à organização.",
      "Não use o chip de filtro da busca no lugar do chip de pessoa."
    ],
    "conteudo": "O chip mostra o nome e, na dica, o endereço. Endereços de e-mail podem ter caracteres de qualquer alfabeto.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A leitura de destinatários é testada em <code>src/lib/us4.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "campos",
      "avatar",
      "composicao",
      "busca"
    ],
    "historico": []
  },
  "anexos": {
    "usadoEm": [
      "src/features/message/ConversationView.tsx",
      "src/ui/Attachment.tsx"
    ],
    "fazer": [
      "Use a cor do tipo de arquivo no ícone.",
      "Mostre o progresso no envio e o motivo na falha.",
      "Mostre o tamanho."
    ],
    "nao": [
      "Não abra o arquivo sozinho.",
      "Não esconda a falha: diga o motivo.",
      "Não aceite anexo acima do limite sem avisar."
    ],
    "conteudo": "O tamanho segue o formato do idioma. O nome do arquivo é do usuário e não é traduzido.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "composicao",
      "leitura",
      "carregamento"
    ],
    "historico": []
  },
  "etiquetas": {
    "usadoEm": [
      "src/ui/Thread.tsx"
    ],
    "fazer": [
      "Use as seis cores, cada uma com um par de fundo e texto de 4,5:1 nos dois temas.",
      "Dê a cada etiqueta um nome, além da cor.",
      "Deixe a pessoa escolher a cor."
    ],
    "nao": [
      "Não use vermelho nem amarelo: significam erro e aviso.",
      "Não crie etiquetas só por cor.",
      "Não misture fundo de um par com o texto de outro."
    ],
    "conteudo": "O nome da etiqueta é da pessoa e não é traduzido.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. O contraste é medido ao vivo em <a href=\"acessibilidade.html\">Acessibilidade</a>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "avatar",
      "pastas",
      "lozenge"
    ],
    "historico": []
  },
  "busca": {
    "usadoEm": [
      "src/features/mailbox/Mailbox.tsx",
      "src/features/search/ApplyRuleDialog.tsx",
      "src/layout/CommandBar.tsx",
      "src/layout/EmailListColumn.tsx",
      "src/layout/FoldersColumn.tsx",
      "src/layout/HeaderSearch.tsx",
      "src/layout/SearchResultsBar.tsx",
      "src/ui/Thread.tsx"
    ],
    "fazer": [
      "Transforme o que a pessoa digita em chips que ela vê e edita.",
      "Aceite operadores e sugestões; os dois dão o mesmo chip.",
      "Busque nas mensagens guardadas no computador, sem rede."
    ],
    "nao": [
      "Não mande o texto da busca para a internet.",
      "Não troque a pessoa do chip sem avisar quando há mais de um contato com o nome.",
      "Não esconda 'Salvar busca' nem 'Todas as contas'."
    ],
    "conteudo": "Os operadores (<code>de:</code>, <code>para:</code>, <code>depois:</code>) são traduzidos junto com a interface. Datas aceitam o formato do idioma.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A busca é testada em <code>src/lib/us16.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica: a busca é local em qualquer sistema.",
    "relacionados": [
      "padrao-busca",
      "campos",
      "destinatarios",
      "threads"
    ],
    "historico": [
      [
        "US16",
        "Chips, todas as contas e regras nos resultados (L73 a L75)."
      ]
    ]
  },
  "regras": {
    "usadoEm": [
      "src/features/rules/RuleDialogs.tsx",
      "src/features/rules/RuleEditor.tsx",
      "src/features/rules/RuleHistory.tsx",
      "src/features/rules/RuleList.tsx",
      "src/features/rules/RulesSettings.tsx",
      "src/ui/RuleSentence.tsx"
    ],
    "fazer": [
      "Mostre cada regra como uma frase, na lista, no editor e no histórico.",
      "Mostre a prévia antes de salvar.",
      "Registre tudo que a regra faz, com 'Desfazer'."
    ],
    "nao": [
      "Não aplique uma regra sem prévia.",
      "Não exclua o que a regra pega: excluir vai para a Lixeira.",
      "Não esconda por que uma regra foi desligada."
    ],
    "conteudo": "A frase é montada por partes; ao traduzir, monte a frase inteira por idioma, não junte pedaços.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A frase e os campos são testados em <code>src/lib/us12.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "busca",
      "toasts",
      "modais",
      "campos"
    ],
    "historico": [
      [
        "US12",
        "Regras (L58 a L63)."
      ]
    ]
  },
  "arquivos": {
    "usadoEm": [
      "src/App.tsx",
      "src/features/accounts/AddAccount.tsx",
      "src/features/import/ArchiveSettings.tsx",
      "src/features/import/ImportDialog.tsx",
      "src/layout/AccountMenu.tsx",
      "src/layout/FoldersColumn.tsx",
      "src/ui/AccountAvatar.tsx",
      "src/ui/DropZone.tsx",
      "src/ui/ImportSummary.tsx",
      "src/ui/Modal.tsx"
    ],
    "fazer": [
      "Diga o que o app reconheceu e quanto vai ocupar antes de começar.",
      "Mostre o progresso com 'Cancelar'.",
      "Reconheça o formato pelo conteúdo, não só pela extensão."
    ],
    "nao": [
      "Não precise de conta nem de internet para abrir o arquivo.",
      "Não mostre o conteúdo dos e-mails nos erros.",
      "Não use o nome do programa de origem fora da ajuda de onde achar o backup."
    ],
    "conteudo": "Os nomes dos programas de origem aparecem só onde a pessoa os procura. O resto do texto fala de 'arquivo de e-mail'.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. A importação é testada em <code>src/lib/us15.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Arrastar um arquivo para a janela e o diálogo de escolha usam o sistema; o resultado é o mesmo no Windows e no Linux.",
    "relacionados": [
      "contas",
      "modais",
      "carregamento",
      "vazios"
    ],
    "historico": [
      [
        "US15",
        "Importação de arquivos (L70 a L72)."
      ]
    ]
  },
  "criptografia": {
    "usadoEm": [
      "src/features/crypto/AddKeyDialog.tsx",
      "src/features/crypto/CertManager.tsx",
      "src/features/crypto/CryptoSettings.tsx",
      "src/features/crypto/KeyManager.tsx",
      "src/features/crypto/SmimeDialogs.tsx",
      "src/features/import/ImportDialog.tsx",
      "src/features/message/ComposeCard.tsx",
      "src/features/message/ConversationView.tsx",
      "src/features/settings/Settings.tsx",
      "src/ui/ChoiceCard.tsx",
      "src/ui/ComposeSecurityBar.tsx",
      "src/ui/Fingerprint.tsx",
      "src/ui/InlineCompose.tsx",
      "src/ui/PemBox.tsx",
      "src/ui/SecurityStatus.tsx"
    ],
    "fazer": [
      "Mostre cada conceito de segurança com texto, ícone e forma próprios.",
      "Use verde só para assinatura verificada com chave aceita e conferida.",
      "Use 'criptografar', 'assinar', 'chave' e 'certificado' na interface; siglas só onde a pessoa precisa."
    ],
    "nao": [
      "Não use um único indicador verde para conceitos diferentes.",
      "Não dependa só da cor para o estado de segurança.",
      "Não afirme o que o sistema não consegue comprovar."
    ],
    "conteudo": "Linguagem simples para quem não é técnico. Impressão digital em grupos, sempre em fonte mono. Os estados têm frase própria, nunca 'seguro' sozinho.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. As chaves e o envio criptografado são testados em <code>src/lib/us13.test.ts</code> e <code>src/lib/us14.test.ts</code>. Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "As chaves ficam no cofre do sistema: Credential Manager no Windows, Secret Service (GNOME Keyring ou KWallet) no Linux.",
    "relacionados": [
      "leitura",
      "composicao",
      "mensagens",
      "modais"
    ],
    "historico": [
      [
        "US13",
        "OpenPGP (L64 a L69)."
      ],
      [
        "US14",
        "S/MIME."
      ],
      [
        "2026-10-28",
        "Estados de segurança distintos (v3 §2.6)."
      ]
    ]
  },
  "documentacao": {
    "usadoEm": [],
    "fazer": [
      "Use o gradiente só no início de páginas de destaque.",
      "Mantenha tudo local: sem CDN, sem fonte remota.",
      "Deixe o idioma a um clique, em todas as páginas."
    ],
    "nao": [
      "Não ponha gradiente atrás de texto longo.",
      "Não use o gradiente no app de e-mail."
    ],
    "conteudo": "As docs existem em cinco idiomas.",
    "testes": "Automático: <code>pnpm ds:check</code> confere tokens, temas e o registro de páginas. O site de docs tem build próprio no CI (<code>docs.yml</code>). Teclado, foco e os dois temas são conferidos na mão; ainda não existe teste visual automático.",
    "plataforma": "Não se aplica.",
    "relacionados": [
      "layouts",
      "governanca"
    ],
    "historico": [
      [
        "L77–L80",
        "Gradientes, ícones de marca, idioma e timeline."
      ]
    ]
  }
};

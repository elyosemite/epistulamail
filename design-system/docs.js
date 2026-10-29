/* Epistula Design System · docs.js
   Casca compartilhada da documentação: ícones, índice, paginação, tema, toasts, tooltips
   e as demonstrações interativas (cada uma só roda na página que tem o elemento dela).
   Para adicionar uma página: crie <id>.html com <body data-page="<id>"> e registre em EPISTULA_PAGES. */
/* Maturidade (v3 §9): draft, experimental, beta, stable, deprecated, removed. Quarto item de cada página. */
const EPISTULA_STATUS = {
  draft: ['Rascunho', ''],
  experimental: ['Experimental', ' lozenge--moved'],
  beta: ['Beta', ' lozenge--inprogress'],
  stable: ['Estável', ' lozenge--success'],
  deprecated: ['Descontinuado', ' lozenge--overdue'],
  removed: ['Removido', ' lozenge--removed']
};
const maturityBadge = (st, extra = '') => { const [label, cls] = EPISTULA_STATUS[st] || EPISTULA_STATUS.stable; return `<span class="lozenge${cls}${extra}">${label}</span>`; };
const EPISTULA_PAGES = [
  {
    "group": "Fundamentos",
    "items": [
      [
        "principios",
        "Princípios",
        "As quatro regras que orientam cada decisão.",
        "stable"
      ],
      [
        "cores",
        "Cores",
        "Tokens semânticos de texto, superfície, borda e ícone.",
        "stable"
      ],
      [
        "tipografia",
        "Tipografia",
        "Escala de Hero a Label, pesos e famílias.",
        "stable"
      ],
      [
        "espacamento",
        "Espaçamento e layout",
        "Escala de 2 a 80px e o layout de três painéis.",
        "stable"
      ],
      [
        "forma",
        "Forma e elevação",
        "Raios e os quatro níveis de superfície.",
        "stable"
      ],
      [
        "acessibilidade",
        "Acessibilidade",
        "Contraste medido ao vivo e atalhos de teclado.",
        "stable"
      ],
      [
        "variaveis",
        "Variáveis CSS",
        "Todas as custom properties de tokens.css, por nível.",
        "stable"
      ],
      [
        "temas",
        "Temas e densidade",
        "Claro, escuro, alto contraste, densidade e a cor de destaque da conta.",
        "stable"
      ],
      [
        "conteudo",
        "Conteúdo e tradução",
        "Tom de voz, microtexto, tradução e leitura da direita para a esquerda.",
        "stable"
      ],
      [
        "arquitetura",
        "Arquitetura",
        "Camadas, regra de classificação, primitives, utilities e a ordem de adoção.",
        "stable"
      ],
      [
        "governanca",
        "Governança",
        "Camadas, maturidade, gates de revisão e definição de pronto.",
        "stable"
      ]
    ]
  },
  {
    "group": "Core",
    "items": [
      [
        "botoes",
        "Botões",
        "Primário, padrão, sutil, perigo, aviso e link.",
        "stable"
      ],
      [
        "campos",
        "Campos de texto",
        "Caixa padrão, busca e linha de composição.",
        "stable"
      ],
      [
        "selecao",
        "Seleção",
        "Checkbox, toggle, provedores e cor da conta.",
        "stable"
      ],
      [
        "avatar",
        "Avatar",
        "Pessoas e contas, com inicial, cor e foto.",
        "stable"
      ],
      [
        "badge",
        "Badge",
        "Contagens, como não lidas na pasta.",
        "stable"
      ],
      [
        "lozenge",
        "Lozenge",
        "Status de uma mensagem, em uma palavra.",
        "stable"
      ],
      [
        "menus",
        "Menus e tooltips",
        "Listas de ações e rótulos de ícones.",
        "stable"
      ],
      [
        "mensagens",
        "Banners e avisos",
        "Alertas do sistema e avisos em contexto.",
        "stable"
      ],
      [
        "toasts",
        "Toasts",
        "Confirmações com Desfazer.",
        "stable"
      ],
      [
        "modais",
        "Modais",
        "Decisões que não têm desfazer.",
        "stable"
      ],
      [
        "navegacao",
        "Navegação lateral",
        "Painel e item de navegação, sem nada de e-mail.",
        "stable"
      ]
    ]
  },
  {
    "group": "Padrões",
    "items": [
      [
        "padrao-validacao",
        "Validação de formulário",
        "Campos, mensagens de erro e resumo de erros.",
        "beta"
      ],
      [
        "padrao-confirmacao",
        "Confirmação",
        "Ação sem volta, ação com Desfazer e quando não confirmar.",
        "beta"
      ],
      [
        "padrao-busca",
        "Busca",
        "Campo, sugestões, filtros, resultados e nenhum resultado.",
        "beta"
      ],
      [
        "padrao-selecao-em-lote",
        "Seleção em lote",
        "Marcar itens, contador, ações em lote e limpar a seleção.",
        "beta"
      ],
      [
        "vazios",
        "Empty states",
        "Caixa zerada, busca sem resultado e erro.",
        "stable"
      ],
      [
        "carregamento",
        "Carregamento",
        "Skeleton, spinner e progresso.",
        "stable"
      ]
    ]
  },
  {
    "group": "Produto",
    "items": [
      [
        "contas",
        "Contas",
        "Coluna de contas e troca de conta.",
        "stable"
      ],
      [
        "pastas",
        "Pastas",
        "Pastas, etiquetas, armazenamento, arrastar e sinalizadas.",
        "stable"
      ],
      [
        "header",
        "Header e busca",
        "Barra global e busca com operadores.",
        "stable"
      ],
      [
        "threads",
        "Lista de threads",
        "Linhas, estados e ações em massa.",
        "stable"
      ],
      [
        "leitura",
        "Painel de leitura",
        "Assunto, mensagem e conversa recolhida.",
        "stable"
      ],
      [
        "composicao",
        "Composição",
        "Janela de nova mensagem com desfazer envio.",
        "stable"
      ],
      [
        "destinatarios",
        "Chips de destinatário",
        "Destinatários internos, externos e inválidos.",
        "stable"
      ],
      [
        "anexos",
        "Anexos",
        "Cartões de arquivo, envio e erro.",
        "stable"
      ],
      [
        "etiquetas",
        "Etiquetas",
        "As seis cores de etiqueta do usuário.",
        "stable"
      ],
      [
        "busca",
        "Busca",
        "Pessoas, período e filtros como chips; buscas salvas.",
        "stable"
      ],
      [
        "regras",
        "Regras",
        "Organizar os e-mails assim que chegam.",
        "stable"
      ],
      [
        "arquivos",
        "Arquivos de e-mail",
        "Abrir backups de outros programas, sem conta.",
        "beta"
      ],
      [
        "criptografia",
        "Criptografia",
        "Chaves, certificados e mensagens criptografadas.",
        "beta"
      ]
    ]
  },
  {
    "group": "Layouts e páginas",
    "items": [
      [
        "layouts",
        "Layouts",
        "As regiões da casca, da caixa de entrada, da busca, das configurações e da conta.",
        "stable"
      ],
      [
        "documentacao",
        "Documentação",
        "Gradientes, ícones de marca, idioma, timeline e o layout de docs.epistulamail.com.",
        "stable"
      ]
    ]
  }
];

(() => {
  document.body.insertAdjacentHTML('afterbegin', "<svg width=\"0\" height=\"0\" style=\"position:absolute\" aria-hidden=\"true\">\n  <symbol id=\"i-inbox\" viewBox=\"0 0 24 24\"><path d=\"M22 12h-6l-2 3h-4l-2-3H2\"/><path d=\"M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z\"/></symbol>\n  <symbol id=\"i-star\" viewBox=\"0 0 24 24\"><path d=\"M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01z\"/></symbol>\n  <symbol id=\"i-send\" viewBox=\"0 0 24 24\"><path d=\"m22 2-7 20-4-9-9-4Z\"/><path d=\"M22 2 11 13\"/></symbol>\n  <symbol id=\"i-file\" viewBox=\"0 0 24 24\"><path d=\"M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z\"/><path d=\"M14 2v6h6\"/></symbol>\n  <symbol id=\"i-archive\" viewBox=\"0 0 24 24\"><rect x=\"2\" y=\"3\" width=\"20\" height=\"5\" rx=\"1\"/><path d=\"M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8\"/><path d=\"M10 12h4\"/></symbol>\n  <symbol id=\"i-trash\" viewBox=\"0 0 24 24\"><path d=\"M3 6h18\"/><path d=\"M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6\"/><path d=\"M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2\"/></symbol>\n  <symbol id=\"i-tag\" viewBox=\"0 0 24 24\"><path d=\"M12 2H2v10l9.29 9.29a1 1 0 0 0 1.41 0l8.59-8.59a1 1 0 0 0 0-1.41z\"/><circle cx=\"7\" cy=\"7\" r=\"1.5\"/></symbol>\n  <symbol id=\"i-search\" viewBox=\"0 0 24 24\"><circle cx=\"11\" cy=\"11\" r=\"7\"/><path d=\"m21 21-4.3-4.3\"/></symbol>\n  <symbol id=\"i-clip\" viewBox=\"0 0 24 24\"><path d=\"m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48\"/></symbol>\n  <symbol id=\"i-reply\" viewBox=\"0 0 24 24\"><path d=\"M9 17 4 12l5-5\"/><path d=\"M20 18v-2a4 4 0 0 0-4-4H4\"/></symbol>\n  <symbol id=\"i-forward\" viewBox=\"0 0 24 24\"><path d=\"m15 17 5-5-5-5\"/><path d=\"M4 18v-2a4 4 0 0 1 4-4h12\"/></symbol>\n  <symbol id=\"i-more\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"5\" r=\"1\"/><circle cx=\"12\" cy=\"12\" r=\"1\"/><circle cx=\"12\" cy=\"19\" r=\"1\"/></symbol>\n  <symbol id=\"i-check\" viewBox=\"0 0 24 24\"><path d=\"M20 6 9 17l-5-5\"/></symbol>\n  <symbol id=\"i-x\" viewBox=\"0 0 24 24\"><path d=\"M18 6 6 18M6 6l12 12\"/></symbol>\n  <symbol id=\"i-warning\" viewBox=\"0 0 24 24\"><path d=\"m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3\"/><path d=\"M12 9v4M12 17h.01\"/></symbol>\n  <symbol id=\"i-info\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 16v-4M12 8h.01\"/></symbol>\n  <symbol id=\"i-success\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"m9 12 2 2 4-4\"/></symbol>\n  <symbol id=\"i-error\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"m15 9-6 6M9 9l6 6\"/></symbol>\n  <symbol id=\"i-sparkle\" viewBox=\"0 0 24 24\"><path d=\"M12 3l1.9 5.6L19.5 10l-5.6 1.9L12 17.5l-1.9-5.6L4.5 10l5.6-1.4z\"/><path d=\"M19 3v4M17 5h4\"/></symbol>\n  <symbol id=\"i-spam\" viewBox=\"0 0 24 24\"><path d=\"M7.86 2h8.28L22 7.86v8.28L16.14 22H7.86L2 16.14V7.86z\"/><path d=\"M12 8v4M12 16h.01\"/></symbol>\n  <symbol id=\"i-sliders\" viewBox=\"0 0 24 24\"><path d=\"M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6\"/></symbol>\n  <symbol id=\"i-pencil\" viewBox=\"0 0 24 24\"><path d=\"M12 20h9\"/><path d=\"M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z\"/></symbol>\n  <symbol id=\"i-chevron-down\" viewBox=\"0 0 24 24\"><path d=\"m6 9 6 6 6-6\"/></symbol>\n  <symbol id=\"i-chevron-left\" viewBox=\"0 0 24 24\"><path d=\"m15 18-6-6 6-6\"/></symbol>\n  <symbol id=\"i-chevron-right\" viewBox=\"0 0 24 24\"><path d=\"m9 18 6-6-6-6\"/></symbol>\n  <symbol id=\"i-clock\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 6v6l4 2\"/></symbol>\n  <symbol id=\"i-mail-open\" viewBox=\"0 0 24 24\"><path d=\"M21.2 8.4c.5.38.8.97.8 1.6v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V10a2 2 0 0 1 .8-1.6l8-6a2 2 0 0 1 2.4 0l8 6Z\"/><path d=\"m22 10-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 10\"/></symbol>\n  <symbol id=\"i-mail\" viewBox=\"0 0 24 24\"><rect x=\"2\" y=\"4\" width=\"20\" height=\"16\" rx=\"2\"/><path d=\"m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7\"/></symbol>\n  <symbol id=\"i-folder\" viewBox=\"0 0 24 24\"><path d=\"M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z\"/></symbol>\n  <symbol id=\"i-image\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\"/><circle cx=\"9\" cy=\"9\" r=\"2\"/><path d=\"m21 15-3.09-3.09a2 2 0 0 0-2.82 0L6 21\"/></symbol>\n  <symbol id=\"i-download\" viewBox=\"0 0 24 24\"><path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\"/><path d=\"m7 10 5 5 5-5\"/><path d=\"M12 15V3\"/></symbol>\n  <symbol id=\"i-refresh\" viewBox=\"0 0 24 24\"><path d=\"M21 12a9 9 0 1 1-3-6.7L21 8\"/><path d=\"M21 3v5h-5\"/></symbol>\n  <symbol id=\"i-plus\" viewBox=\"0 0 24 24\"><path d=\"M12 5v14M5 12h14\"/></symbol>\n  <symbol id=\"i-minus\" viewBox=\"0 0 24 24\"><path d=\"M5 12h14\"/></symbol>\n  <symbol id=\"i-expand\" viewBox=\"0 0 24 24\"><path d=\"M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7\"/></symbol>\n  <symbol id=\"i-bold\" viewBox=\"0 0 24 24\"><path d=\"M6 12h9a4 4 0 0 1 0 8H6V4h8a4 4 0 0 1 0 8\"/></symbol>\n  <symbol id=\"i-italic\" viewBox=\"0 0 24 24\"><path d=\"M19 4h-9M14 20H5M15 4 9 20\"/></symbol>\n  <symbol id=\"i-list\" viewBox=\"0 0 24 24\"><path d=\"M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01\"/></symbol>\n  <symbol id=\"i-link\" viewBox=\"0 0 24 24\"><path d=\"M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71\"/><path d=\"M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71\"/></symbol>\n  <symbol id=\"i-help\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3M12 17h.01\"/></symbol>\n  <symbol id=\"i-grid\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"3\" width=\"5\" height=\"5\" rx=\"1\"/><rect x=\"10\" y=\"3\" width=\"5\" height=\"5\" rx=\"1\"/><rect x=\"17\" y=\"3\" width=\"4\" height=\"5\" rx=\"1\"/><rect x=\"3\" y=\"10\" width=\"5\" height=\"5\" rx=\"1\"/><rect x=\"10\" y=\"10\" width=\"5\" height=\"5\" rx=\"1\"/><rect x=\"17\" y=\"10\" width=\"4\" height=\"5\" rx=\"1\"/><rect x=\"3\" y=\"17\" width=\"5\" height=\"4\" rx=\"1\"/><rect x=\"10\" y=\"17\" width=\"5\" height=\"4\" rx=\"1\"/><rect x=\"17\" y=\"17\" width=\"4\" height=\"4\" rx=\"1\"/></symbol>\n  <symbol id=\"i-moon\" viewBox=\"0 0 24 24\"><path d=\"M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z\"/></symbol>\n  <symbol id=\"i-sun\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"4\"/><path d=\"M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41\"/></symbol>\n  <symbol id=\"i-copy\" viewBox=\"0 0 24 24\"><rect x=\"9\" y=\"9\" width=\"13\" height=\"13\" rx=\"2\"/><path d=\"M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1\"/></symbol>\n  <symbol id=\"i-rows\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/><path d=\"M3 10h18M3 15h18\"/></symbol>\n  <symbol id=\"i-eye\" viewBox=\"0 0 24 24\"><path d=\"M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/></symbol>\n  <symbol id=\"i-log-out\" viewBox=\"0 0 24 24\"><path d=\"M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4\"/><path d=\"m16 17 5-5-5-5\"/><path d=\"M21 12H9\"/></symbol>\n  <symbol id=\"i-log-in\" viewBox=\"0 0 24 24\"><path d=\"M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4\"/><path d=\"m10 17 5-5-5-5\"/><path d=\"M15 12H3\"/></symbol>\n  <symbol id=\"i-arrow-up\" viewBox=\"0 0 24 24\"><path d=\"m5 12 7-7 7 7\"/><path d=\"M12 19V5\"/></symbol>\n  <symbol id=\"i-arrow-down\" viewBox=\"0 0 24 24\"><path d=\"M12 5v14\"/><path d=\"m19 12-7 7-7-7\"/></symbol>\n  <symbol id=\"i-droplet\" viewBox=\"0 0 24 24\"><path d=\"M12 2.7 6.3 8.4a8 8 0 1 0 11.4 0z\"/></symbol>\n  <symbol id=\"i-globe\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M2 12h20\"/><path d=\"M12 2a15 15 0 0 1 0 20 15 15 0 0 1 0-20\"/></symbol>\n  <symbol id=\"i-flag\" viewBox=\"0 0 24 24\"><path d=\"M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z\"/><path d=\"M4 22v-7\"/></symbol>\n  <symbol id=\"i-reply-all\" viewBox=\"0 0 24 24\"><path d=\"m7 17-5-5 5-5\"/><path d=\"m12 17-5-5 5-5\"/><path d=\"M22 18v-2a4 4 0 0 0-4-4H7\"/></symbol>\n  <symbol id=\"i-folder-plus\" viewBox=\"0 0 24 24\"><path d=\"M12 10v6M9 13h6\"/><path d=\"M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z\"/></symbol>\n  <symbol id=\"i-monitor\" viewBox=\"0 0 24 24\"><rect x=\"2\" y=\"3\" width=\"20\" height=\"14\" rx=\"2\"/><path d=\"M8 21h8M12 17v4\"/></symbol>\n  <symbol id=\"i-settings\" viewBox=\"0 0 24 24\"><path d=\"M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/></symbol>\n  <symbol id=\"i-user\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"8\" r=\"4\"/><path d=\"M4 21a8 8 0 0 1 16 0\"/></symbol>\n  <symbol id=\"i-palette\" viewBox=\"0 0 24 24\"><path d=\"M12 22a10 10 0 1 1 10-10c0 2.2-1.8 3.5-4 3.5h-1.6a1.9 1.9 0 0 0-1.4 3.2c.6.7.2 3.3-3 3.3z\"/><circle cx=\"7.5\" cy=\"11\" r=\"1\"/><circle cx=\"10.5\" cy=\"7\" r=\"1\"/><circle cx=\"15.5\" cy=\"7.5\" r=\"1\"/></symbol>\n  <symbol id=\"i-folder-move\" viewBox=\"0 0 24 24\"><path d=\"M2 9V5a2 2 0 0 1 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H20a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-1\"/><path d=\"M2 13h10\"/><path d=\"m9 16 3-3-3-3\"/></symbol>\n  <symbol id=\"i-sidebar-left\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\"/><path d=\"M9 3v18\"/></symbol>\n  <symbol id=\"i-sidebar-right\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\"/><path d=\"M15 3v18\"/></symbol>\n  <symbol id=\"i-bell\" viewBox=\"0 0 24 24\"><path d=\"M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9\"/><path d=\"M10.3 21a1.94 1.94 0 0 0 3.4 0\"/></symbol>\n  <symbol id=\"i-bell-off\" viewBox=\"0 0 24 24\"><path d=\"M8.7 3A6 6 0 0 1 18 8a21.3 21.3 0 0 0 .6 5\"/><path d=\"M17 17H3s3-2 3-9a4.67 4.67 0 0 1 .3-1.7\"/><path d=\"M10.3 21a1.94 1.94 0 0 0 3.4 0\"/><path d=\"m2 2 20 20\"/></symbol>\n  <symbol id=\"i-hard-drive\" viewBox=\"0 0 24 24\"><path d=\"M22 12H2\"/><path d=\"M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z\"/><path d=\"M6 16h.01\"/><path d=\"M10 16h.01\"/></symbol>\n  <symbol id=\"i-lock\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"11\" width=\"18\" height=\"11\" rx=\"2\"/><path d=\"M7 11V7a5 5 0 0 1 10 0v4\"/></symbol>\n  <symbol id=\"i-shield\" viewBox=\"0 0 24 24\"><path d=\"M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z\"/></symbol>\n  <symbol id=\"i-key\" viewBox=\"0 0 24 24\"><path d=\"m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4\"/><path d=\"m21 2-9.6 9.6\"/><circle cx=\"7.5\" cy=\"15.5\" r=\"5.5\"/></symbol>\n  <symbol id=\"i-eye-off\" viewBox=\"0 0 24 24\"><path d=\"M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68\"/><path d=\"M6.61 6.61A13.53 13.53 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61\"/><path d=\"m2 2 20 20\"/><path d=\"M14.12 14.12a3 3 0 1 1-4.24-4.24\"/></symbol>\n  <symbol id=\"i-filter\" viewBox=\"0 0 24 24\"><path d=\"M3 4h18l-7 8.5V19l-4 2v-8.5z\"/></symbol>\n  <symbol id=\"i-grip\" viewBox=\"0 0 24 24\"><circle cx=\"9\" cy=\"6\" r=\"1\"/><circle cx=\"15\" cy=\"6\" r=\"1\"/><circle cx=\"9\" cy=\"12\" r=\"1\"/><circle cx=\"15\" cy=\"12\" r=\"1\"/><circle cx=\"9\" cy=\"18\" r=\"1\"/><circle cx=\"15\" cy=\"18\" r=\"1\"/></symbol>\n  <symbol id=\"i-play\" viewBox=\"0 0 24 24\"><path d=\"M6 4l14 8-14 8z\"/></symbol>\n  <symbol id=\"i-arrow-right\" viewBox=\"0 0 24 24\"><path d=\"M5 12h14\"/><path d=\"m13 6 6 6-6 6\"/></symbol>\n  <symbol id=\"i-history\" viewBox=\"0 0 24 24\"><path d=\"M3 12a9 9 0 1 0 3-6.7L3 8\"/><path d=\"M3 3v5h5\"/><path d=\"M12 7v5l3 2\"/></symbol>\n  <symbol id=\"i-receipt\" viewBox=\"0 0 24 24\"><path d=\"M5 3h14v18l-3-2-2 2-2-2-2 2-2-2-3 2z\"/><path d=\"M9 8h6M9 12h6\"/></symbol>\n  <symbol id=\"i-lock-open\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"11\" width=\"18\" height=\"11\" rx=\"2\"/><path d=\"M7 11V7a5 5 0 0 1 9.9-1\"/></symbol>\n  <symbol id=\"i-badge-check\" viewBox=\"0 0 24 24\"><path d=\"M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z\"/><path d=\"m9 12 2 2 4-4\"/></symbol>\n  <symbol id=\"i-signature\" viewBox=\"0 0 24 24\"><path d=\"m21 17-2.16-1.87a.5.5 0 0 0-.84.37v.5a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1c0-2.55-4-3.97-8.5-4a1 1 0 0 0 0 5c4.15 0 4.75-11.3 5.71-13.5a2.5 2.5 0 1 1 3.31 3.28\"/><path d=\"M3 21h18\"/></symbol>\n  <symbol id=\"i-upload\" viewBox=\"0 0 24 24\"><path d=\"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4\"/><path d=\"m17 8-5-5-5 5\"/><path d=\"M12 3v12\"/></symbol>\n  <symbol id=\"i-github\" viewBox=\"0 0 24 24\"><path d=\"M12 .5a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2c-3.3.7-4-1.6-4-1.6-.6-1.4-1.4-1.8-1.4-1.8-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C17.3 4.7 18.3 5 18.3 5c.7 1.7.2 2.9.1 3.2.8.8 1.2 1.9 1.2 3.2 0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .5z\"/></symbol>\n  <symbol id=\"i-x-brand\" viewBox=\"0 0 24 24\"><path d=\"M18.2 2h3.4l-7.4 8.5L23 22h-6.8l-5.3-7-6.1 7H1.4l7.9-9L1 2h7l4.8 6.4L18.2 2zm-1.2 18h1.9L7 3.9H5L17 20z\"/></symbol>\n  <symbol id=\"i-discord\" viewBox=\"0 0 24 24\"><path d=\"M20.3 4.4A19.6 19.6 0 0 0 15.4 3l-.6 1.3a18.2 18.2 0 0 0-5.6 0L8.6 3a19.6 19.6 0 0 0-4.9 1.4C.6 9-.3 13.6.1 18.1A19.8 19.8 0 0 0 6.2 21l1.3-2.1c-.7-.3-1.4-.6-2-1l.5-.4a14.1 14.1 0 0 0 12 0l.5.4c-.6.4-1.3.7-2 1l1.3 2.1a19.7 19.7 0 0 0 6.1-2.9c.5-5.2-.8-9.8-3.6-13.7zM8.3 15.4c-1.2 0-2.2-1.1-2.2-2.4s1-2.4 2.2-2.4 2.2 1.1 2.2 2.4-1 2.4-2.2 2.4zm7.4 0c-1.2 0-2.2-1.1-2.2-2.4s1-2.4 2.2-2.4 2.2 1.1 2.2 2.4-1 2.4-2.2 2.4z\"/></symbol>\n  <symbol id=\"i-certificate\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"4\" width=\"18\" height=\"12\" rx=\"2\"/><path d=\"M7 8h10M7 12h5\"/><circle cx=\"16\" cy=\"16\" r=\"3\"/><path d=\"m14.5 18.5-1 3.5 2.5-1.2 2.5 1.2-1-3.5\"/></symbol>\n</svg>");
  const page = document.body.dataset.page || 'index';
  const href = (id) => id + '.html';
  const flat = EPISTULA_PAGES.flatMap(g => g.items.map(([id, title, desc, status]) => ({ id, title, desc, status, group: g.group })));

  document.getElementById('doc-nav').innerHTML =
    `<a class="doc-nav__home${page === 'index' ? ' is-current' : ''}" href="index.html"${page === 'index' ? ' aria-current="page"' : ''}>Visão geral</a>` +
    EPISTULA_PAGES.map(g => `<div class="doc-nav__group"><div class="nav-heading">${g.group}</div>${g.items.map(([id, title]) =>
      `<a href="${href(id)}"${id === page ? ' class="is-current" aria-current="page"' : ''}>${title}</a>`).join('')}</div>`).join('');

  const main = document.querySelector('main');
  const i = flat.findIndex(p => p.id === page);
  if (i >= 0) {
    main.insertAdjacentHTML('afterbegin', `<div class="page-top">
      <a class="wordmark" href="index.html"><img class="wordmark__logo" src="images/epistulamail-logo-120.png" alt="">EpistulaMail</a>
      <span class="crumb">${flat[i].group}</span>
      <button class="btn btn--subtle theme-toggle" id="theme-toggle" type="button" aria-label="Alternar tema"></button>
    </div>`);
    const h1 = main.querySelector('h1');
    if (h1) h1.insertAdjacentHTML('beforeend', maturityBadge(flat[i].status, ' maturity'));
    const prev = flat[i - 1], next = flat[i + 1];
    main.insertAdjacentHTML('beforeend', `<nav class="pager" aria-label="Paginação">
      ${prev ? `<a class="pager__link" href="${href(prev.id)}"><span>Anterior</span><b>${prev.title}</b></a>` : ''}
      ${next ? `<a class="pager__link pager__link--next" href="${href(next.id)}"><span>Próxima</span><b>${next.title}</b></a>` : ''}
    </nav>`);
  }

  const grid = document.getElementById('overview-grid');
  if (grid) grid.innerHTML = EPISTULA_PAGES.map(g => `<section class="ov-group"><h2 class="part__title">${g.group}</h2><div class="ov-grid">${g.items.map(([id, title, desc, status]) =>
    `<a class="ov-card" href="${href(id)}"><span class="ov-card__head"><b>${title}</b>${maturityBadge(status)}</span><span>${desc}</span></a>`).join('')}</div></section>`).join('');

  document.body.insertAdjacentHTML('beforeend', '<div class="toast-region" id="toast-region" aria-live="polite"></div>');

  /* ---------------- Ficha do componente (v3 §7) ---------------- */
  /* Os dados ficam em fichas.js (carregado sob demanda). Camada, status e relacionados vêm do registro acima;
     os tokens são lidos das regras de components.css que casam com as classes usadas na página. */
  if (flat.some(p => p.id === page)) {
    const fichaScript = document.createElement('script');
    fichaScript.src = 'fichas.js';
    fichaScript.onload = () => { try { renderFicha(); } catch (e) { /* sem ficha, a página continua inteira */ } };
    document.head.appendChild(fichaScript);
  }
  function renderFicha() {
    const data = (typeof EPISTULA_FICHAS !== 'undefined') && EPISTULA_FICHAS[page];
    if (!data) return;
    const me = flat.find(p => p.id === page);
    const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;');
    const li = (items) => items.map(t => `<li>${t}</li>`).join('');
    const rel = (data.relacionados || []).map(id => flat.find(p => p.id === id)).filter(Boolean)
      .map(p => `<a href="${href(p.id)}">${esc(p.title)}</a>`).join(', ');
    const used = data.usadoEm && data.usadoEm.length
      ? `<details class="ficha__used"><summary>${data.usadoEm.length} ${data.usadoEm.length === 1 ? 'arquivo' : 'arquivos'} em <code>src/</code></summary><ul>${data.usadoEm.map(f => `<li><code>${esc(f)}</code></li>`).join('')}</ul></details>`
      : 'Só nas páginas deste design system e nos sites; não aparece no app.';
    const hist = (data.historico || []).concat([['2026-10-28', 'Ficha do componente (v3): camada, status, fazer e não fazer, tokens, testes e relacionados.']])
      .map(([when, what]) => `<li><b>${esc(when)}</b> ${esc(what)}</li>`).join('');
    const tokens = tokensOnPage();
    const section = document.createElement('section');
    section.className = 'ds-section ficha';
    section.id = 'ficha';
    section.innerHTML = `
      <div class="ds-head"><p class="eyebrow">v3 · Ficha do componente</p><h2>Ficha</h2>
        <p class="lede">O resumo de como esta peça entra no sistema: onde fica, onde é usada, o que fazer e evitar, e como é verificada.</p></div>
      <dl class="guidance ficha__meta">
        <div><dt>Camada</dt><dd>${esc(me.group)}</dd></div>
        <div><dt>Status</dt><dd>${maturityBadge(me.status)}</dd></div>
        <div><dt>Usado em</dt><dd>${used}</dd></div>
        <div><dt>Anatomia e API</dt><dd>As classes e as variantes estão na linha "Classes" ao fim de cada seção; os estados, nos exemplos.</dd></div>
      </dl>
      <div class="ficha__dos">
        <div class="ficha__do"><h3><svg class="i"><use href="#i-check"/></svg>Fazer</h3><ul>${li(data.fazer)}</ul></div>
        <div class="ficha__dont"><h3><svg class="i"><use href="#i-x"/></svg>Não fazer</h3><ul>${li(data.nao)}</ul></div>
      </div>
      <div class="ficha__tokens"><h3>Tokens usados</h3>${tokens}</div>
      <dl class="guidance">
        <div><dt>Conteúdo e tradução</dt><dd>${data.conteudo}</dd></div>
        <div><dt>Testes</dt><dd>${data.testes}</dd></div>
        <div><dt>Notas de plataforma</dt><dd>${data.plataforma}</dd></div>
        <div><dt>Relacionados</dt><dd>${rel || 'Nenhum.'}</dd></div>
        <div><dt>Histórico</dt><dd><ul class="ficha__hist">${hist}</ul></dd></div>
      </dl>`;
    const pager = main.querySelector('.pager');
    if (pager) pager.before(section); else main.appendChild(section);
    if (location.hash === '#ficha') section.scrollIntoView();
  }
  /* Tokens semânticos citados pelas regras de components.css que casam com as classes desta página. */
  function tokensOnPage() {
    const classes = new Set();
    main.querySelectorAll('.ds-section:not(.ficha) [class]').forEach(el => el.classList.forEach(c => classes.add(c)));
    const sheet = [...document.styleSheets].find(s => (s.href || '').endsWith('components.css'));
    let rules;
    try { rules = sheet.cssRules; } catch (e) { return 'Abra o design system por http para ver a lista.'; }
    const found = new Set();
    const walk = (list) => {
      for (const r of list) {
        if (r.cssRules && !r.selectorText) { walk(r.cssRules); continue; }
        if (!r.selectorText) continue;
        const cls = [...r.selectorText.matchAll(/\.([\w-]+)/g)].map(m => m[1]);
        if (!cls.some(c => classes.has(c))) continue;
        for (const m of r.cssText.matchAll(/var\((--[\w-]+)/g)) found.add(m[1]);
      }
    };
    walk(rules);
    const cs = getComputedStyle(document.documentElement);
    const names = [...found].filter(n => cs.getPropertyValue(n).trim()).sort();   // só os que estão em :root (tokens de verdade)
    if (!names.length) return 'Nenhum token do :root: usa só valores locais da própria peça.';
    const groups = [['Cor', /^--color-/], ['Espaço', /^--space-/], ['Tipografia', /^--(font|line-height|letter-spacing)-/], ['Forma e borda', /^--(radius|border)-/],
      ['Elevação', /^--shadow-/], ['Layout e densidade', /^--(layout|row-height|control-height|input-height)/], ['Foco, opacidade e ícones', /^--(focus|opacity|icon)-/], ['Movimento e camadas', /^--(duration|easing|z-)/], ['Gradientes', /^--gradient-/]];
    const taken = new Set();
    const html = groups.map(([label, re]) => {
      const list = names.filter(n => re.test(n)); list.forEach(n => taken.add(n));
      return list.length ? `<div class="ficha__tk-group"><b>${label}</b><span class="ficha__tk-list">${list.map(n => `<a class="ficha__tk" href="variaveis.html#tk-${n}"><code>${n}</code></a>`).join('')}</span></div>` : '';
    }).join('');
    const rest = names.filter(n => !taken.has(n));
    return `<p class="ficha__tk-n">${names.length} tokens do sistema aparecem nos exemplos desta página. Clique para ver o valor.</p>${html}${rest.length ? `<div class="ficha__tk-group"><b>Outros</b><span class="ficha__tk-list">${rest.map(n => `<a class="ficha__tk" href="variaveis.html#tk-${n}"><code>${n}</code></a>`).join('')}</span></div>` : ''}`;
  }
})();

(() => {
  const root = document.documentElement;
  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => [...el.querySelectorAll(s)];
  const icon = (name, cls = '') => `<svg class="i ${cls}"><use href="#i-${name}"/></svg>`;
  const cssVar = (name) => getComputedStyle(root).getPropertyValue(name).trim();

  /* ---------------- Tema ---------------- */
  const themeBtn = $('#theme-toggle');
  const isDark = () => root.dataset.theme ? root.dataset.theme === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
  function syncThemeButton() {
    const dark = isDark();
    themeBtn.innerHTML = icon(dark ? 'sun' : 'moon') + `<span>${dark ? 'Tema claro' : 'Tema escuro'}</span>`;
  }
  themeBtn.addEventListener('click', () => {
    root.dataset.theme = isDark() ? 'light' : 'dark';
    try { localStorage.setItem('epistula-ds-theme', root.dataset.theme); } catch (e) {}
    syncThemeButton(); refreshThemeValues();
  });
  try { const saved = localStorage.getItem('epistula-ds-theme'); if (saved) root.dataset.theme = saved; } catch (e) {}
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => { syncThemeButton(); refreshThemeValues(); });
  new MutationObserver(() => { syncThemeButton(); refreshThemeValues(); }).observe(root, { attributes: true, attributeFilter: ['data-theme'] });

  /* ---------------- Cores ---------------- */
  const swatchGroups = [
    { title: 'Texto', kind: 'text', tokens: [
      ['--color-text', 'Remetente, assunto, corpo'], ['--color-text-subtle', 'Prévia, rótulos de campo'], ['--color-text-subtlest', 'Datas, metadados, placeholder'],
      ['--color-text-disabled', 'Controles desabilitados'], ['--color-text-brand', 'Data de não lida, destaques'], ['--color-link', 'Links e botão link'],
      ['--color-text-danger', 'Erros, "Rascunho" na lista'], ['--color-text-warning', 'Avisos em fundo sutil'], ['--color-text-success', 'Confirmações'],
      ['--color-text-information', 'Informação'], ['--color-text-discovery', 'Novidades'], ['--color-text-inverse', 'Texto sobre fundos fortes'] ] },
    { title: 'Superfícies', kind: 'bg', tokens: [
      ['--color-surface', 'Lista e painel de leitura'], ['--color-surface-sunken', 'Fundo do app, sidebar'], ['--color-surface-raised', 'Cartões, anexos'],
      ['--color-surface-overlay', 'Menus, toasts, modais'], ['--color-blanket', 'Véu atrás de modais'] ] },
    { title: 'Fundos de componente', kind: 'bg', tokens: [
      ['--color-bg-brand-bold', 'Botão primário'], ['--color-bg-brand-bold-hovered', 'Primário em hover'], ['--color-bg-selected', 'Linha marcada, pasta atual'],
      ['--color-bg-selected-hovered', 'Selecionado em hover'], ['--color-bg-neutral', 'Botão padrão, chip'], ['--color-bg-neutral-hovered', 'Padrão em hover'],
      ['--color-bg-neutral-subtle-hovered', 'Hover de sutil e nav'], ['--color-bg-neutral-bold', 'Tooltip, banner de anúncio'], ['--color-bg-danger-bold', 'Botão de perigo'],
      ['--color-bg-danger', 'Aviso de erro'], ['--color-bg-warning-bold', 'Banner offline'], ['--color-bg-warning', 'Aviso de atenção'],
      ['--color-bg-success-bold', 'Toggle ligado'], ['--color-bg-success', 'Aviso de sucesso'], ['--color-bg-information', 'Aviso informativo'], ['--color-bg-discovery', 'Novidade'] ] },
    { title: 'Bordas', kind: 'border', tokens: [
      ['--color-border', 'Divisórias, contorno de cartões'], ['--color-border-bold', 'Divisórias fortes'], ['--color-border-input', 'Campos de formulário'],
      ['--color-border-focused', 'Anel de foco'], ['--color-border-selected', 'Barra da conversa aberta'], ['--color-border-danger', 'Campo inválido'] ] },
    { title: 'Ícones', kind: 'icon', tokens: [
      ['--color-icon', 'Ícones de ação'], ['--color-icon-subtle', 'Ícones secundários'], ['--color-icon-brand', 'Ponto de não lida'],
      ['--color-icon-starred', 'Estrela'], ['--color-icon-danger', 'Erro'], ['--color-icon-warning', 'Atenção'], ['--color-icon-success', 'Sucesso'] ] },
    { title: 'Etiquetas', kind: 'label', tokens: ['blue', 'teal', 'green', 'purple', 'magenta', 'orange'].map(c => [`--color-label-${c}-bg`, c]) },
  ];
  const labelNames = { blue: 'Novidades', teal: 'Dev', green: 'Financeiro', purple: 'Jurídico', magenta: 'Pessoal', orange: 'Clientes' };

  function swatchChip(kind, token, extra) {
    if (kind === 'text') return `<span class="swatch__chip" style="color:var(${token});background:${token === '--color-text-inverse' ? 'var(--color-bg-neutral-bold)' : 'var(--color-surface)'}">Aa</span>`;
    if (kind === 'border') return `<span class="swatch__chip" style="background:var(--color-surface)"><span style="width:60%;height:60%;border:2px solid var(${token});border-radius:var(--radius-small)"></span></span>`;
    if (kind === 'icon') return `<span class="swatch__chip" style="color:var(${token});background:var(--color-surface)">${icon(token.includes('starred') ? 'star' : token.includes('danger') ? 'error' : token.includes('warning') ? 'warning' : token.includes('success') ? 'success' : 'inbox', 'i--xl')}</span>`;
    if (kind === 'label') return `<span class="swatch__chip" style="background:var(${token})"><span class="label label--${extra}" style="font-size:var(--font-size-body)">${labelNames[extra]}</span></span>`;
    return `<span class="swatch__chip" style="background:var(${token})"></span>`;
  }
  if ($('#swatches')) $('#swatches').innerHTML = swatchGroups.map(g => `
    <div class="swatch-group"><h3 class="sub" style="margin:0">${g.title}</h3><div class="swatch-grid">
      ${g.tokens.map(([t, use]) => {
        const isLabel = g.kind === 'label';
        const varName = isLabel ? `${t} · ${t.replace('-bg', '-text')}` : t;
        return `<button class="swatch" type="button" data-copy="${isLabel ? `var(${t}); color: var(${t.replace('-bg', '-text')})` : `var(${t})`}" aria-label="Copiar ${t}">
          ${swatchChip(g.kind, t, use)}
          <span class="swatch__info"><span class="swatch__var">${varName}</span><span class="swatch__hex" data-hex="${t}" ${isLabel ? `data-hex2="${t.replace('-bg', '-text')}"` : ''}></span>${isLabel ? '' : `<span class="swatch__use">${use}</span>`}</span>
        </button>`;
      }).join('')}
    </div></div>`).join('');

  const ramps = [
    ['Azul (marca)', ['#E9F2FF','#CCE0FF','#85B8FF','#579DFF','#388BFF','#1D7AFC','#0C66E4','#0055CC','#09326C','#092957']],
    ['Neutro', ['#F7F8F9','#F1F2F4','#DCDFE4','#B3B9C4','#8590A2','#758195','#626F86','#44546F','#2C3E5D','#172B4D']],
    ['Vermelho', ['#FFECEB','#FFD5D2','#FD9891','#F87168','#F15B50','#E2483D','#C9372C','#AE2E24','#5D1F1A','#42221F']],
    ['Amarelo e laranja', ['#FFF7D6','#F8E6A0','#F5CD47','#E2B203','#CF9F02','#FEA362','#F38A3F','#E56910','#A54800','#533F04']],
    ['Verde', ['#DCFFF1','#BAF3DB','#7EE2B8','#4BCE97','#2ABB7F','#22A06B','#1F845A','#216E4E','#164B35','#1C3329']],
    ['Roxo', ['#F3F0FF','#DFD8FD','#B8ACF6','#9F8FEF','#8F7EE7','#8270DB','#6E5DC6','#5E4DB2','#352C63','#2B273F']],
  ];
  const lum = (hex) => {
    const [r, g, b] = [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255).map(c => c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  if ($('#ramps')) $('#ramps').innerHTML = ramps.map(([name, hexes]) => `
    <div class="ramp-label"><span>${name}</span><span>100 → 1000</span></div>
    <div class="ramp">${hexes.map(h => `<span style="background:${h};color:${lum(h) > 0.35 ? '#172B4D' : '#FFFFFF'}" title="${h}">${h.slice(1)}</span>`).join('')}</div>`).join('');

  /* hex8 → rgb composto sobre a superfície atual */
  function parseColor(val, over) {
    val = val.trim();
    if (val.startsWith('var(')) val = cssVar(val.slice(4, -1));
    const m = val.match(/^#([0-9a-f]{6})([0-9a-f]{2})?$/i);
    if (!m) return null;
    let rgb = [0, 2, 4].map(i => parseInt(m[1].slice(i, i + 2), 16));
    const a = m[2] ? parseInt(m[2], 16) / 255 : 1;
    if (a < 1 && over) rgb = rgb.map((c, i) => Math.round(c * a + over[i] * (1 - a)));
    return rgb;
  }
  const toHex = (rgb) => '#' + rgb.map(c => c.toString(16).padStart(2, '0')).join('').toUpperCase();
  const relLum = (rgb) => { const [r, g, b] = rgb.map(c => c / 255).map(c => c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
  const ratio = (a, b) => { const [l1, l2] = [relLum(a), relLum(b)].sort((x, y) => y - x); return (l1 + 0.05) / (l2 + 0.05); };

  const contrastPairs = [
    ['--color-text', '--color-surface', 4.5, 'Aa'], ['--color-text-subtle', '--color-surface', 4.5, 'Aa'], ['--color-text-subtlest', '--color-surface', 4.5, 'Aa'],
    ['--color-text-subtlest', '--color-surface-sunken', 4.5, 'Aa'], ['--color-text-brand', '--color-surface', 4.5, 'Aa'], ['--color-text-selected', '--color-bg-selected', 4.5, 'Aa'],
    ['--color-text-inverse', '--color-bg-brand-bold', 4.5, 'Aa'], ['--color-text-inverse', '--color-bg-danger-bold', 4.5, 'Aa'], ['--color-text-warning-inverse', '--color-bg-warning-bold', 4.5, 'Aa'],
    ['--color-text-inverse', '--color-bg-neutral-bold', 4.5, 'Aa'], ['--color-text-inverse', '--color-bg-success-bold', 4.5, 'Aa'],
    ['--color-text-danger', '--color-bg-danger', 4.5, 'Aa'], ['--color-text-warning', '--color-bg-warning', 4.5, 'Aa'], ['--color-text-success', '--color-bg-success', 4.5, 'Aa'],
    ['--color-text-information', '--color-bg-information', 4.5, 'Aa'], ['--color-text-discovery', '--color-bg-discovery', 4.5, 'Aa'],
    ...['blue', 'teal', 'green', 'purple', 'magenta', 'orange'].map(c => [`--color-label-${c}-text`, `--color-label-${c}-bg`, 4.5, 'Aa']),
    ['--color-border-input', '--color-bg-input', 3, '▢'], ['--color-border-focused', '--color-surface', 3, '▢'],
    ['--color-icon', '--color-surface', 3, '●'], ['--color-icon-brand', '--color-surface', 3, '●'], ['--color-icon-starred', '--color-surface', 3, '★'],
  ];

  function refreshThemeValues() {
    const surface = parseColor(cssVar('--color-surface'));
    $$('[data-hex]').forEach(el => {
      const raw = cssVar(el.dataset.hex);
      const raw2 = el.dataset.hex2 ? cssVar(el.dataset.hex2) : '';
      el.textContent = raw2 ? `${raw} · ${raw2}` : raw;
    });
    if ($('#contrast-rows')) $('#contrast-rows').innerHTML = contrastPairs.map(([fg, bg, min, glyph]) => {
      const bgRgb = parseColor(cssVar(bg), surface);
      const fgRgb = parseColor(cssVar(fg), bgRgb);
      if (!bgRgb || !fgRgb) return '';
      const r = ratio(fgRgb, bgRgb);
      const pass = r >= min;
      return `<tr>
        <td><span class="pair" style="color:${toHex(fgRgb)};background:${toHex(bgRgb)}">${glyph}</span></td>
        <td><code>${fg}</code></td><td><code>${bg}</code></td>
        <td class="num">${r.toFixed(2).replace('.', ',')}:1</td>
        <td>${min === 3 ? '3:1 (não texto)' : '4,5:1 (texto)'}</td>
        <td><span class="lozenge ${pass ? 'lozenge--success' : 'lozenge--removed'}">${pass ? (r >= (min === 3 ? 4.5 : 7) ? 'AAA' : 'AA') : 'Falha'}</span></td>
      </tr>`;
    }).join('');
  }

  /* ---------------- Espaçamento ---------------- */
  const spaces = [
    ['025', 2, 'Respiro interno de chips e badges'], ['050', 4, 'Entre botões de ícone; padding de lozenge'], ['075', 6, 'Remetente ↔ contagem da conversa'],
    ['100', 8, 'Padding de itens de navegação; colunas da linha'], ['150', 12, 'Padding vertical da linha; gutter do header'], ['200', 16, 'Padding horizontal da linha; section message'],
    ['250', 20, 'Entre grupos de campos'], ['300', 24, 'Padding do painel de leitura e de modais'], ['400', 32, 'Entre mensagens expandidas da conversa'],
    ['500', 40, 'Entre seções de configurações'], ['600', 48, 'Padding de empty states'], ['800', 64, 'Margem de páginas de configuração'], ['1000', 80, 'Topo de onboarding'],
  ];
  if ($('#space-rows')) $('#space-rows').innerHTML = spaces.map(([t, px, use]) => `<tr><td><code>--space-${t}</code></td><td>${(px / 16).toString().replace('.', ',')}rem</td><td>${px}px</td><td><span class="space-bar" style="width:var(--space-${t})"></span></td><td>${use}</td></tr>`).join('');

  /* ---------------- Toasts ---------------- */
  const region = $('#toast-region');
  function toast({ type = 'success', icon: ic, title, desc, actions = [], timeout = 6000, timer = 0, onTimeout }) {
    const el = document.createElement('div');
    el.className = `toast toast--${type}`;
    el.setAttribute('role', type === 'error' ? 'alert' : 'status');
    const iconName = ic || { success: 'success', info: 'info', warning: 'warning', error: 'error' }[type];
    el.innerHTML = `${icon(iconName)}<div><p class="toast__title">${title}</p>${desc ? `<p class="toast__desc">${desc}</p>` : ''}${actions.length ? `<div class="toast__actions">${actions.map((a, i) => `<button type="button" data-i="${i}">${a.label}</button>`).join('')}</div>` : ''}</div>
      <button class="btn btn--subtle btn--icon btn--compact" type="button" aria-label="Fechar">${icon('x')}</button>${timer ? '<div class="progress toast__timer"><span style="width:100%"></span></div>' : ''}`;
    let done = false, handle;
    const close = () => { if (done) return; done = true; clearTimeout(handle); el.remove(); };
    el.querySelector('[aria-label="Fechar"]').onclick = close;
    el.querySelectorAll('[data-i]').forEach(b => b.onclick = () => { close(); actions[b.dataset.i].fn?.(); });
    region.prepend(el);
    if (timer) {
      const bar = el.querySelector('.toast__timer > span');
      requestAnimationFrame(() => requestAnimationFrame(() => { bar.style.transition = `width ${timer}ms linear`; bar.style.width = '0%'; }));
      handle = setTimeout(() => { close(); onTimeout?.(); }, timer);
    } else if (type !== 'error' && timeout) {
      handle = setTimeout(close, timeout);
    }
    while (region.children.length > 3) region.lastElementChild.remove();
    return close;
  }

  function undoSend() {
    toast({ type: 'info', icon: 'send', title: 'Enviando em 5 s', desc: 'Para: Marina Couto, joao@lumen.com.br', timer: 5000,
      actions: [{ label: 'Desfazer', fn: () => toast({ type: 'info', icon: 'file', title: 'Envio cancelado', desc: 'A mensagem voltou para Rascunhos.' }) }],
      onTimeout: () => toast({ title: 'Mensagem enviada', actions: [{ label: 'Ver mensagem' }] }) });
  }
  $$('[data-toast]').forEach(b => b.addEventListener('click', () => {
    const k = b.dataset.toast;
    if (k === 'archive') toast({ title: 'Conversa arquivada', actions: [{ label: 'Desfazer', fn: () => toast({ type: 'info', icon: 'inbox', title: 'A conversa voltou para a Caixa de entrada' }) }] });
    if (k === 'send') undoSend();
    if (k === 'error') toast({ type: 'error', title: 'A mensagem não foi enviada', desc: 'O servidor SMTP não respondeu. Ela ficou na Caixa de saída.', actions: [{ label: 'Tentar de novo', fn: undoSend }] });
  }));
  $('#compose-demo')?.addEventListener('submit', (e) => { e.preventDefault(); undoSend(); });
  $('#sidebar-compose')?.addEventListener('click', () => { location.href = 'composicao.html'; });

  /* ---------------- Coluna de contas e cor da conta ---------------- */
  $$('[data-rail-demo]').forEach((shell) => {
    const header = $('[data-rail-account]', shell);
    const items = $$('.account-rail__item', shell);
    items.forEach((btn, i) => {
      btn.addEventListener('click', () => {
        items.forEach((b) => b.removeAttribute('aria-current'));
        btn.setAttribute('aria-current', 'true');
        const [name, email, color, initial] = btn.dataset.account.split('|');
        const cls = color === 'blue' ? '' : ` avatar--${color}`;
        header.innerHTML = `<span class="avatar avatar--sm${cls}" aria-hidden="true">${initial}</span><span class="appbar__account-text"><span class="appbar__account-name">${name}</span> · ${email}</span>`;
      });
      btn.addEventListener('keydown', (e) => {
        const next = e.key === 'ArrowDown' ? items[i + 1] : e.key === 'ArrowUp' ? items[i - 1] : null;
        if (next) { e.preventDefault(); next.focus(); }
      });
    });
  });
  $$('[data-account-setup]').forEach((modal) => {
    const input = $('input:not([type])', modal);
    const chips = $$('[data-name]', modal);
    const swatches = $$('.color-swatch .avatar', modal);
    const update = () => {
      const initial = (input.value.trim()[0] || '?').toUpperCase();
      swatches.forEach((a) => { a.textContent = initial; });
      chips.forEach((c) => c.classList.toggle('is-selected', c.dataset.name === input.value.trim()));
    };
    input.addEventListener('input', update);
    chips.forEach((c) => c.addEventListener('click', () => { input.value = c.dataset.name; update(); }));
  });

  /* ---------------- Divisor da lista e filtro do "Mover para" ---------------- */
  $$('[data-resize-demo]').forEach((shell) => {
    const handle = $('[data-resizer]', shell);
    const list = $('.app-shell__list', shell);
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
    const MIN = 20 * rem, MAX = 35 * rem, DEFAULT = 25 * rem;
    const set = (px) => {
      const w = Math.max(MIN, Math.min(MAX, px));
      shell.style.setProperty('--app-shell-list', `${w}px`);
      handle.setAttribute('aria-valuenow', String(Math.round(((w - MIN) / (MAX - MIN)) * 100)));
    };
    handle.addEventListener('pointerdown', (e) => {
      handle.setPointerCapture(e.pointerId);
      handle.classList.add('is-dragging');
      const start = e.clientX, from = list.getBoundingClientRect().width;
      const move = (ev) => set(from + ev.clientX - start);
      const up = () => { handle.classList.remove('is-dragging'); handle.removeEventListener('pointermove', move); handle.removeEventListener('pointerup', up); };
      handle.addEventListener('pointermove', move);
      handle.addEventListener('pointerup', up);
    });
    handle.addEventListener('keydown', (e) => {
      const w = list.getBoundingClientRect().width, step = e.shiftKey ? 64 : 16;
      if (e.key === 'ArrowLeft') set(w - step);
      else if (e.key === 'ArrowRight') set(w + step);
      else if (e.key === 'Home') set(MIN);
      else if (e.key === 'End') set(MAX);
      else return;
      e.preventDefault();
    });
    handle.addEventListener('dblclick', () => set(DEFAULT));
  });
  $$('[data-move-menu]').forEach((menu) => {
    const search = $('[data-move-search]', menu);
    const items = $$('[data-folders] .menu-item', menu);
    const empty = $('[data-empty]', menu);
    search.addEventListener('input', () => {
      const q = search.value.trim().toLowerCase();
      let shown = 0;
      items.forEach((b) => { const ok = b.textContent.toLowerCase().includes(q); b.hidden = !ok; if (ok) shown++; });
      empty.hidden = shown > 0;
    });
  });

  /* ---------------- Modal ---------------- */
  const openModalBtn = $('#open-modal');
  openModalBtn?.addEventListener('click', () => {
    const blanket = document.createElement('div');
    blanket.className = 'blanket';
    blanket.innerHTML = `<div class="modal" role="dialog" aria-modal="true" aria-labelledby="live-modal-title">
      <div class="modal__head">${icon('warning')}<h3 class="modal__title" id="live-modal-title">Esvaziar a lixeira?</h3></div>
      <div class="modal__body"><p>As 128 conversas da lixeira serão excluídas permanentemente. Essa ação não pode ser desfeita.</p></div>
      <div class="modal__foot"><button class="btn btn--subtle" type="button" data-act="cancel">Cancelar</button><button class="btn btn--danger" type="button" data-act="confirm">Esvaziar lixeira</button></div></div>`;
    document.body.appendChild(blanket);
    const btns = $$('button', blanket);
    btns[0].focus();
    const close = () => { blanket.remove(); document.removeEventListener('keydown', onKey); openModalBtn.focus(); };
    const onKey = (e) => {
      if (e.key === 'Escape') close();
      if (e.key === 'Tab') { const i = btns.indexOf(document.activeElement); e.preventDefault(); btns[(i + (e.shiftKey ? -1 : 1) + btns.length) % btns.length].focus(); }
    };
    document.addEventListener('keydown', onKey);
    blanket.addEventListener('click', (e) => {
      if (e.target === blanket || e.target.dataset.act === 'cancel') close();
      if (e.target.dataset.act === 'confirm') { close(); toast({ title: 'Lixeira esvaziada', desc: '128 conversas foram excluídas permanentemente.' }); }
    });
  });

  /* ---------------- Menu ---------------- */
  const menuBtn = $('#menu-trigger'), menu = $('#move-menu');
  if (menu) {
  const setMenu = (open) => { menu.hidden = !open; menuBtn.setAttribute('aria-expanded', open); if (open) $('.menu-item', menu).focus(); };
  menuBtn.addEventListener('click', (e) => { e.stopPropagation(); setMenu(menu.hidden); });
  menu.addEventListener('click', (e) => {
    const item = e.target.closest('.menu-item'); if (!item) return;
    $$('.menu-item', menu).forEach(m => m.setAttribute('aria-checked', m === item));
    setMenu(false); menuBtn.focus();
    toast({ type: 'info', icon: 'folder', title: `Movida para ${item.textContent.trim().replace(/E$/, '').trim()}` });
  });
  menu.addEventListener('keydown', (e) => {
    const items = $$('.menu-item', menu), i = items.indexOf(document.activeElement);
    if (e.key === 'ArrowDown') { e.preventDefault(); items[(i + 1) % items.length].focus(); }
    if (e.key === 'ArrowUp') { e.preventDefault(); items[(i - 1 + items.length) % items.length].focus(); }
    if (e.key === 'Escape') { setMenu(false); menuBtn.focus(); }
  });
  document.addEventListener('click', (e) => { if (!menu.hidden && !menu.contains(e.target)) setMenu(false); });
  }

  /* ---------------- Tooltip: fica dentro do card e da janela, e vira para cima se faltar espaço ---------------- */
  const tip = document.createElement('div');
  tip.className = 'tooltip tooltip--floating';
  tip.id = 'floating-tip';
  tip.setAttribute('role', 'tooltip');
  tip.hidden = true;
  document.body.appendChild(tip);
  let tipTarget = null;
  function showTip(el) {
    tipTarget = el;
    tip.textContent = el.dataset.tip;
    tip.hidden = false;
    const r = el.getBoundingClientRect(), t = tip.getBoundingClientRect();
    const box = (el.closest('.stage, .modal, .menu, .toast') || root).getBoundingClientRect();
    const pad = 8, gap = 6;
    const minX = Math.max(box.left + pad, pad);
    const maxX = Math.min(box.right - pad, innerWidth - pad) - t.width;
    const x = Math.max(minX, Math.min(r.left + r.width / 2 - t.width / 2, maxX));
    let y = r.bottom + gap;
    if (y + t.height > Math.min(box.bottom - pad, innerHeight - pad)) y = r.top - gap - t.height;
    tip.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
    el.setAttribute('aria-describedby', 'floating-tip');
  }
  function hideTip() {
    tipTarget?.removeAttribute('aria-describedby');
    tipTarget = null;
    tip.hidden = true;
  }
  document.addEventListener('pointerover', (e) => {
    const el = e.target.closest('[data-tip]');
    if (el && el !== tipTarget) showTip(el);
    else if (!el && tipTarget) hideTip();
  });
  document.addEventListener('pointerout', (e) => { if (!e.relatedTarget) hideTip(); });
  document.addEventListener('focusin', (e) => { const el = e.target.closest('[data-tip]'); if (el && el.matches(':focus-visible')) showTip(el); });
  document.addEventListener('focusout', hideTip);
  document.addEventListener('click', hideTip);
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape') hideTip(); });
  addEventListener('scroll', hideTip, true);

  /* ---------------- Checkbox indeterminado ---------------- */
  $$('[data-indeterminate]').forEach(c => c.indeterminate = true);

  /* ---------------- Chips de destinatário ---------------- */
  const ORG = 'epistula.com.br';
  const contacts = { 'marina.couto@epistula.com.br': 'Marina Couto', 'rafael.dias@epistula.com.br': 'Rafael Dias' };
  const avatarTone = ['purple', 'teal', 'green', 'magenta', 'blue'];
  let recipients = ['marina.couto@epistula.com.br', 'joao@lumen.com.br', 'ana.lima@'];
  let selectedChip = -1;
  const chipBox = $('#chip-box'), chipEntry = $('#chip-entry'), chipHelp = $('#chip-help');
  const validEmail = (s) => /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(s);
  function renderChips() {
    $$('.chip', chipBox).forEach(c => c.remove());
    recipients.forEach((addr, i) => {
      const valid = validEmail(addr), external = valid && !addr.endsWith('@' + ORG);
      const name = contacts[addr] || addr;
      const initials = (contacts[addr] ? name.split(' ').map(w => w[0]).join('') : addr[0]).slice(0, 2);
      const chip = document.createElement('span');
      chip.className = `chip${!valid ? ' is-invalid' : external ? ' is-external' : ''}${i === selectedChip ? ' is-selected' : ''}`;
      chip.title = addr;
      chip.innerHTML = `${valid ? `<span class="avatar avatar--${external ? 'orange' : avatarTone[i % avatarTone.length]}">${initials}</span>` : icon('warning', 'i--sm')}<span class="chip__name">${name}</span><button class="chip__remove" type="button" aria-label="Remover ${name}">${icon('x', 'i--sm')}</button>`;
      chip.querySelector('button').onclick = () => { recipients.splice(i, 1); selectedChip = -1; renderChips(); chipEntry.focus(); };
      chipBox.insertBefore(chip, chipEntry);
    });
    const invalid = recipients.filter(r => !validEmail(r)).length;
    const external = recipients.filter(r => validEmail(r) && !r.endsWith('@' + ORG)).length;
    chipHelp.className = 'field__help' + (invalid ? ' field__help--error' : '');
    chipHelp.innerHTML = invalid
      ? `${icon('error', 'i--sm')}${invalid === 1 ? '1 endereço inválido. Corrija-o para enviar.' : `${invalid} endereços inválidos. Corrija-os para enviar.`}`
      : external
        ? `${icon('warning', 'i--sm')}${external === 1 ? '1 destinatário é' : `${external} destinatários são`} de fora de @${ORG}.`
        : 'Todos os destinatários são da sua organização.';
    chipBox.classList.toggle('is-invalid', invalid > 0);
  }
  function commitEntry() {
    const v = chipEntry.value.replace(/[,;\s]+$/, '').trim();
    if (v) { recipients.push(v); chipEntry.value = ''; selectedChip = -1; renderChips(); }
  }
  if (chipBox) {
  chipEntry.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ',' || e.key === ';') { e.preventDefault(); commitEntry(); }
    else if (e.key === 'Backspace' && !chipEntry.value && recipients.length) {
      e.preventDefault();
      if (selectedChip === recipients.length - 1) { recipients.pop(); selectedChip = -1; } else selectedChip = recipients.length - 1;
      renderChips();
    } else if (selectedChip !== -1) { selectedChip = -1; renderChips(); }
  });
  chipEntry.addEventListener('blur', commitEntry);
  chipBox.addEventListener('click', (e) => { if (e.target === chipBox) chipEntry.focus(); });
  renderChips();
  }

  /* ---------------- Lista de threads ---------------- */
  const seed = () => [
    { id: 1, from: 'Marina Couto', count: 3, subject: 'Revisão do contrato: Fornecedor Lumen', snippet: 'Segue a versão 3 com os ajustes da cláusula 7.2. Preciso da sua aprovação até quinta.', date: '10:42', unread: true, attach: true, labels: [['purple', 'Jurídico']] },
    { id: 2, from: 'GitHub', subject: '[epistula/web] PR #482: Corrige sincronização IMAP em pastas grandes', snippet: 'rafael-dias pediu sua revisão. 4 arquivos alterados, +128 −41.', date: '09:15', unread: true, labels: [['teal', 'Dev']] },
    { id: 3, from: 'Rafael Dias', subject: 'Plantão do fim de semana', snippet: 'Consegue trocar comigo o sábado? Fico com o domingo de 2 de novembro.', date: '08:03', unread: true },
    { id: 4, from: 'Yuri Melo', draft: true, subject: 'Proposta Q4 para a diretoria', snippet: 'Olá a todos, como combinamos na reunião de segunda, segue o resumo…', date: 'Ontem' },
    { id: 5, from: 'Banco Atlântico', subject: 'Seu extrato de outubro está disponível', snippet: 'Acesse o extrato consolidado da conta empresarial final 4471.', date: '21 out', attach: true, labels: [['green', 'Financeiro']] },
    { id: 6, from: 'Paulo Henrique, eu', count: 4, subject: 'Almoço de sexta?', snippet: 'Fechado então, 12:30 no lugar de sempre.', date: '20 out', starred: true },
    { id: 7, from: 'Equipe Epistula', subject: 'Novo no Epistula: adiar conversas', snippet: 'Tire uma conversa da caixa e traga de volta na hora certa.', date: '18 out', labels: [['blue', 'Novidades']] },
  ];
  let threads = seed(), checked = new Set(), openId = null;
  const list = $('#thread-list');

  function threadHTML(t, extra = '', caption = '') {
    const cls = ['thread', t.unread ? 'is-unread' : '', extra].filter(Boolean).join(' ');
    const qa = (n, l, a) => `<button class="btn btn--subtle btn--icon btn--compact" type="button" data-act="${a}" aria-label="${l}" data-tip="${l}">${icon(n)}</button>`;
    return `<div class="${cls}" data-id="${t.id}" tabindex="0" aria-label="${t.unread ? 'Não lida, ' : ''}${t.from}, ${t.subject}, ${t.date}"${caption ? ` title="${caption}"` : ''}>
      <input type="checkbox" class="checkbox thread__check" id="chk-${t.id}" aria-label="Selecionar: ${t.subject}">
      <button class="star-btn thread__star" type="button" aria-pressed="${!!t.starred}" aria-label="Marcar com estrela">${icon('star')}</button>
      <div class="thread__from"><span class="thread__from-name">${t.from}</span>${t.count ? `<span class="thread__count">${t.count}</span>` : ''}${caption ? `<span class="lozenge">${caption}</span>` : ''}</div>
      <div class="thread__meta">${t.attach ? icon('clip') : ''}<span class="thread__date">${t.date}</span><span class="thread__actions">${qa('archive', 'Arquivar', 'archive')}${qa('trash', 'Excluir', 'delete')}${qa(t.unread ? 'mail-open' : 'mail', t.unread ? 'Marcar como lida' : 'Marcar como não lida', 'toggle-read')}${qa('clock', 'Adiar', 'snooze')}</span></div>
      <div class="thread__subject">${t.draft ? '<span class="thread__draft">Rascunho</span>' : ''}${t.status || ''}<span class="thread__subject-text">${t.subject}</span>${(t.labels || []).map(([c, n]) => `<span class="label label--${c}">${n}</span>`).join('')}</div>
      <div class="thread__snippet">${t.snippet}</div>
    </div>`;
  }

  function renderList() {
    list.innerHTML = threads.length ? threads.map(t => threadHTML(t, [checked.has(t.id) ? 'is-checked' : '', openId === t.id ? 'is-open' : ''].filter(Boolean).join(' '))).join('')
      : `<div class="empty"><svg class="empty__art" viewBox="0 0 96 96" fill="none" aria-hidden="true"><rect x="12" y="28" width="72" height="48" rx="6" stroke="currentColor" stroke-width="2.5"/><path d="M12 56h20l6 8h20l6-8h20" stroke="currentColor" stroke-width="2.5" stroke-linejoin="round"/><path d="m38 40 7 7 13-14" stroke="var(--color-icon-success)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/></svg><h3 class="empty__title">Tudo em dia</h3><p class="empty__desc">Você organizou todas as conversas deste exemplo.</p><div class="empty__actions"><button class="btn" type="button" data-reset>Restaurar exemplo</button></div></div>`;
    $$('.thread', list).forEach(row => { const id = +row.dataset.id; $('.thread__check', row).checked = checked.has(id); });
    updateToolbar();
  }

  const master = $('#master-check');
  function updateToolbar() {
    const n = checked.size;
    $('#toolbar').classList.toggle('is-selecting', n > 0);
    $('#tb-default').hidden = n > 0;
    $('#tb-selection').hidden = n === 0;
    $('#tb-count').textContent = n === 1 ? '1 selecionada' : `${n} selecionadas`;
    master.checked = n > 0 && n === threads.length;
    master.indeterminate = n > 0 && n < threads.length;
    const unread = threads.filter(t => t.unread).length;
    $('#unread-badge').textContent = unread;
    $('#unread-badge').hidden = unread === 0;
  }

  function removeThreads(ids, one, many) {
    const before = threads.slice();
    threads = threads.filter(t => !ids.includes(t.id));
    ids.forEach(id => checked.delete(id));
    if (ids.includes(openId)) openId = null;
    renderList();
    toast({ title: ids.length === 1 ? `Conversa ${one}` : `${ids.length} conversas ${many}`, actions: [{ label: 'Desfazer', fn: () => { threads = before; renderList(); toast({ type: 'info', icon: 'inbox', title: 'Ação desfeita' }); } }] });
  }
  function act(action, ids) {
    if (!ids.length) return;
    if (action === 'archive') removeThreads(ids, 'arquivada', 'arquivadas');
    if (action === 'delete') removeThreads(ids, 'movida para a lixeira', 'movidas para a lixeira');
    if (action === 'spam') removeThreads(ids, 'denunciada como spam', 'denunciadas como spam');
    if (action === 'snooze') removeThreads(ids, 'adiada até amanhã, 08:00', 'adiadas até amanhã, 08:00');
    if (action === 'read') { threads.forEach(t => { if (ids.includes(t.id)) t.unread = false; }); checked.clear(); renderList(); }
    if (action === 'toggle-read') { threads.forEach(t => { if (ids.includes(t.id)) t.unread = !t.unread; }); renderList(); }
    if (action === 'label') toast({ type: 'info', icon: 'tag', title: `Etiqueta "Jurídico" aplicada a ${ids.length === 1 ? '1 conversa' : ids.length + ' conversas'}` });
  }

  if (list) {
  list.addEventListener('click', (e) => {
    if (e.target.closest('[data-reset]')) { reset(); return; }
    const row = e.target.closest('.thread'); if (!row) return;
    const id = +row.dataset.id, t = threads.find(x => x.id === id);
    if (e.target.closest('.thread__check')) { e.target.checked ? checked.add(id) : checked.delete(id); row.classList.toggle('is-checked', e.target.checked); updateToolbar(); return; }
    if (e.target.closest('.star-btn')) { t.starred = !t.starred; e.target.closest('.star-btn').setAttribute('aria-pressed', t.starred); return; }
    const a = e.target.closest('[data-act]');
    if (a) { act(a.dataset.act, [id]); return; }
    t.unread = false; openId = id; renderList();
    $(`.thread[data-id="${id}"]`, list)?.focus();
  });
  list.addEventListener('keydown', (e) => {
    const row = e.target.closest('.thread'); if (!row || e.target !== row) return;
    const rows = $$('.thread', list), i = rows.indexOf(row), id = +row.dataset.id, t = threads.find(x => x.id === id);
    const k = e.key.toLowerCase();
    if (k === 'j' || e.key === 'ArrowDown') { e.preventDefault(); rows[Math.min(i + 1, rows.length - 1)].focus(); }
    else if (k === 'k' || e.key === 'ArrowUp') { e.preventDefault(); rows[Math.max(i - 1, 0)].focus(); }
    else if (k === 'x') { e.preventDefault(); checked.has(id) ? checked.delete(id) : checked.add(id); renderList(); $(`.thread[data-id="${id}"]`, list).focus(); }
    else if (k === 's') { e.preventDefault(); t.starred = !t.starred; renderList(); $(`.thread[data-id="${id}"]`, list).focus(); }
    else if (k === 'e') { e.preventDefault(); const next = rows[i + 1] || rows[i - 1]; act('archive', [id]); if (next) $(`.thread[data-id="${next.dataset.id}"]`, list)?.focus(); }
    else if (e.key === 'Enter') { e.preventDefault(); t.unread = false; openId = id; renderList(); $(`.thread[data-id="${id}"]`, list).focus(); }
  });
  master.addEventListener('change', () => { if (master.checked) threads.forEach(t => checked.add(t.id)); else checked.clear(); renderList(); });
  $$('[data-bulk]').forEach(b => b.addEventListener('click', () => act(b.dataset.bulk, [...checked])));
  function reset() { threads = seed(); checked.clear(); openId = null; renderList(); }
  $('#reset-list').addEventListener('click', reset);
  $$('[data-density]').forEach(b => b.addEventListener('click', () => {
    $$('[data-density]').forEach(x => x.setAttribute('aria-pressed', x === b));
    list.classList.toggle('thread-list--standard', b.dataset.density === 'standard');
    list.classList.toggle('thread-list--compact', b.dataset.density === 'compact');
  }));
  renderList();
  }


  /* US7 · Colunas recolhíveis, menu da conta, segmentado e citação recolhida (qualquer página que tenha) */
  document.addEventListener('click', (e) => {
    const radio = e.target.closest('.segmented [role="radio"]');
    if (radio) radio.parentElement.querySelectorAll('[role="radio"]').forEach((b) => b.setAttribute('aria-checked', String(b === radio)));
    const column = e.target.closest('[data-toggle-column]');
    if (column) {
      const shell = column.closest('.app-shell');
      const folders = column.dataset.toggleColumn === 'folders';
      const hidden = shell.classList.toggle(folders ? 'app-shell--no-folders' : 'app-shell--no-reader');
      column.setAttribute('aria-pressed', String(!hidden));
      column.dataset.tip = `${hidden ? 'Mostrar' : 'Esconder'} ${folders ? 'pastas (Ctrl+B)' : 'leitura (Ctrl+Alt+B)'}`;
    }
    const trigger = e.target.closest('.account-trigger[aria-controls]');
    document.querySelectorAll('.account-trigger[aria-controls]').forEach((t) => {
      const menu = document.getElementById(t.getAttribute('aria-controls'));
      if (!menu) return;
      const open = t === trigger ? menu.hidden : (menu.contains(e.target) && !menu.hidden);
      menu.hidden = !open;
      t.setAttribute('aria-expanded', String(open));
    });
    const quote = e.target.closest('.compose__quote-toggle');
    if (quote) {
      const target = quote.nextElementSibling;
      if (target) { target.hidden = !target.hidden; quote.setAttribute('aria-expanded', String(!target.hidden)); }
    }
  });
  document.addEventListener('keydown', (e) => {
    const shell = document.getElementById('blocks-shell');
    if (!shell) return;
    if (e.ctrlKey && e.key.toLowerCase() === 'b') { e.preventDefault(); shell.querySelector(`[data-toggle-column="${e.altKey ? 'reader' : 'folders'}"]`)?.click(); }
    if (e.key === 'Escape') shell.querySelectorAll('.account-trigger[aria-expanded="true"]').forEach((t) => t.click());
  });
  /* Galeria estática de estados */
  const base = { id: 0, from: 'Marina Couto', count: 3, subject: 'Revisão do contrato: Fornecedor Lumen', snippet: 'Segue a versão 3 com os ajustes da cláusula 7.2.', date: '10:42', attach: true, labels: [['purple', 'Jurídico']] };
  const states = [
    [{ ...base }, '', 'Lida'],
    [{ ...base, unread: true }, '', 'Não lida'],
    [{ ...base, unread: true }, 'is-hover', 'Hover'],
    [{ ...base }, 'is-checked', 'Marcada'],
    [{ ...base }, 'is-open', 'Aberta'],
    [{ ...base, unread: true }, 'is-focus', 'Foco'],
    [{ ...base, starred: true }, '', 'Com estrela'],
    [{ ...base, from: 'Para: Lumen Jurídico', count: 0, attach: false, labels: [], status: '<span class="lozenge lozenge--inprogress">Enviando</span>', date: 'agora' }, 'is-sending', 'Enviando'],
    [{ ...base, from: 'Para: ana.lima@lumen.com', count: 0, attach: false, labels: [], status: '<span class="lozenge lozenge--removed">Falhou</span>', date: '10:40' }, '', 'Falha'],
  ];
  if ($('#thread-states')) $('#thread-states').innerHTML = states.map(([t, cls, cap], i) => threadHTML({ ...t, id: 900 + i }, cls || 'state-row', cap)).join('');
  $$('#thread-states .thread').forEach(r => { r.removeAttribute('tabindex'); r.style.cursor = 'default'; });
  $$('#thread-states input, #thread-states button').forEach(el => el.tabIndex = -1);
  $$('#thread-states .thread.is-checked .thread__check').forEach(c => c.checked = true);

  /* ---------------- Copiar ---------------- */
  async function copy(text, label) {
    try { await navigator.clipboard.writeText(text); toast({ type: 'info', icon: 'copy', title: `${label} copiado`, timeout: 2500 }); }
    catch (e) {
      const sel = getSelection(), pre = $('#tokens-source');
      if (label === 'tokens.css') { const r = document.createRange(); r.selectNodeContents(pre); sel.removeAllRanges(); sel.addRange(r); }
      toast({ type: 'warning', title: 'Não foi possível copiar automaticamente', desc: label === 'tokens.css' ? 'O texto está selecionado. Use Ctrl+C.' : text, timeout: 5000 });
    }
  }
  $('#swatches')?.addEventListener('click', (e) => { const s = e.target.closest('[data-copy]'); if (s) copy(s.dataset.copy, s.dataset.copy); });

  /* ---------------- Catálogo de tokens (variaveis.html) ---------------- */
  const TOKEN_BLURBS = {
    'cor · texto': 'Cor do texto e dos links. O texto comum usa --color-text; subtle e subtlest são para apoio e metadados. Os de estado (perigo, aviso, sucesso) sempre vêm junto de uma palavra, nunca só a cor.',
    'cor · superfícies': 'Os níveis de profundidade da tela: o fundo afundado, a superfície comum, a elevada e a que flutua sobre as outras (menus e modais). O papel é o fundo claro do corpo de e-mails de terceiros, nos dois temas.',
    'cor · fundos de componente': 'Fundo de campos, botões, seleção e avisos, com as variações de passar o mouse (hovered) e pressionar (pressed). Use o par do mesmo componente para os estados ficarem coerentes.',
    'cor · bordas': 'Linhas de campos, divisores e o anel de foco. A borda de foco é a única que marca o teclado e precisa de contraste 3:1.',
    'cor · ícones': 'Cor dos ícones por significado. O ícone de estado acompanha a cor do texto do mesmo estado.',
    'cor · etiquetas de e-mail': 'Pares de fundo e texto das cores de etiqueta e dos avatares. Use sempre os dois do mesmo par.',
    'elevação · sombras': 'Duas sombras: raised para cartões e blocos, overlay para o que flutua sobre a tela.',
    'tipografia · famílias': 'Inter para a interface e JetBrains Mono para código e chaves. Vêm empacotadas no app, sem pedir nada à internet.',
    'tipografia · pesos': 'Quatro pesos. O 600 marca ênfase e itens não lidos.',
    'tipografia · escala': 'Tamanho, altura de linha e espaçamento entre letras de cada estilo, de Hero a Code.',
    'tipografia · atalhos': 'O estilo completo em uma linha, para usar com font: var(--font-body). Prefira o atalho a combinar tamanho e peso na mão.',
    'espaçamento': 'Escala de base 8, com meios-passos de 2 e 4. O número é o valor em centésimos de rem: 100 = 8px, 050 = 4px, 200 = 16px.',
    'forma': 'Raios de borda e larguras de linha. O raio cresce com o tamanho da peça: checkbox pequeno, modal grande, avatar redondo.',
    'layout do cliente': 'Larguras e alturas da casca do app: colunas, lista, leitura e barras.',
    'camadas': 'A ordem de sobreposição. Use só estes níveis, nunca um número solto.',
    'movimento': 'Durações e curvas. Com "reduzir movimento" ligado no sistema, as durações vão a zero.',
    'densidade': 'Altura das linhas da lista e dos controles. Confortável e Padrão mostram mais linhas por conversa; Compacta mostra uma.',
    'foco': 'O anel de teclado, igual em todo componente: largura, cor e o quanto ele se afasta da borda.',
    'opacidade': 'Só estes níveis de opacidade. Nenhum componente usa um valor solto.',
    'ícones': 'Tamanho do traço dos ícones do sprite, de pequeno (dentro de chips) a extra grande (estados vazios).',
    'breakpoints': 'Larguras em que o layout muda. CSS não aceita variável dentro de @media; estes valores são a referência, e o pnpm ds:check confere se os @media do components.css batem com eles.',
    'gradientes': 'Só para o início de páginas de destaque, nunca atrás de texto longo nem no app de e-mail. Acompanham o tema.'
  };
  function parseTokens(text) {
    const out = [], lines = text.split('\n');
    let inRoot = false, group = '', pending = '';
    const grad = /\/\* L77 · Gradientes[\s\S]*?\*\/\s*:root\s*\{/.exec(text);
    const gradLine = grad ? text.slice(0, grad.index).split('\n').length : -1;
    lines.forEach((line, i) => {
      if (/^:root\s*\{\s*$/.test(line)) { inRoot = true; pending = ''; if (gradLine > 0 && i + 1 >= gradLine) group = 'Gradientes'; return; }
      if (inRoot && /^\}/.test(line)) { inRoot = false; return; }
      if (!inRoot) return;
      const head = /^\s*\/\*\s*-{3,}\s*(.+?)\s*-{3,}\s*\*\/\s*$/.exec(line);
      if (head) { group = head[1]; pending = ''; return; }
      const lone = /^\s*\/\*\s*(.+?)\s*\*\/\s*$/.exec(line);
      if (lone) { pending = lone[1]; return; }
      const trailing = /\/\*\s*(.+?)\s*\*\/\s*$/.exec(line);
      const code = line.replace(/\/\*.*?\*\//g, '');
      const re = /(--[\w-]+)\s*:\s*([^;]+);/g;
      let m, any = false;
      while ((m = re.exec(code))) { any = true; out.push({ name: m[1], raw: m[2].trim(), group, desc: (trailing && trailing[1]) || pending }); }
      if (any) pending = '';
    });
    return out;
  }
  function tokenPreview(t) {
    const n = t.name;
    if (n.startsWith('--gradient-')) return `<span class="tk-swatch tk-swatch--wide" style="background:var(${n})"></span>`;
    if (n.startsWith('--color-') && !/-(width|size)$/.test(n)) return `<span class="tk-swatch" style="background:var(${n})"></span>`;
    if (n.startsWith('--shadow-')) return `<span class="tk-box" style="box-shadow:var(${n})"></span>`;
    if (n.startsWith('--radius-')) return `<span class="tk-box tk-box--line" style="border-radius:var(${n})"></span>`;
    if (n.startsWith('--space-')) return `<span class="tk-space" style="width:var(${n})"></span>`;
    if (n.startsWith('--font-family-')) return `<span class="tk-text" style="font-family:var(${n})">Aa</span>`;
    if (n.startsWith('--font-weight-')) return `<span class="tk-text" style="font-weight:var(${n})">Aa</span>`;
    if (n.startsWith('--font-size-')) return `<span class="tk-text" style="font-size:var(${n});line-height:1">Aa</span>`;
    if (/^--font-(hero|page-title|section|card|body|body-large|body-small|label|code)$/.test(n)) return `<span class="tk-text" style="font:var(${n})">Aa</span>`;
    return '';
  }
  function buildCatalog(text) {
    const host = $('#token-catalog'); if (!host) return;
    const tokens = parseTokens(text);
    if (!tokens.length) { host.innerHTML = '<p class="token-loading">Não foi possível montar o catálogo. Veja o código-fonte abaixo.</p>'; return; }
    const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');
    const groups = [];
    tokens.forEach(t => { let g = groups.find(x => x.name === t.group); if (!g) groups.push(g = { name: t.group, items: [] }); g.items.push(t); });
    const clean = (name) => name.replace(/\s*\(.*$/, '');
    const blurbOf = (name) => /^paleta/i.test(name) ? 'Nível 1: valor bruto, sem intenção de uso. Só tokens.css usa a paleta; componentes e telas usam os tokens semânticos, que apontam para ela. O sufixo -aNN é a opacidade em %.' : TOKEN_BLURBS[clean(name).toLowerCase()];
    host.innerHTML = groups.map(g => `<section class="tk-group" data-group>
      <h3 class="tk-group__title">${esc(clean(g.name))}<span class="tk-group__n">${g.items.length}</span></h3>
      ${blurbOf(g.name) ? `<p class="tk-group__blurb">${esc(blurbOf(g.name))}</p>` : ''}
      <table class="tk-table"><thead><tr><th scope="col" class="tk-prev"><span class="visually-hidden">Prévia</span></th><th scope="col">Token</th><th scope="col">Valor agora</th></tr></thead><tbody>
      ${g.items.map(t => `<tr id="tk-${t.name}" data-token="${t.name}" data-search="${esc((t.name + ' ' + t.desc + ' ' + g.name).toLowerCase())}"><td class="tk-prev">${tokenPreview(t)}</td><td><button class="tk-name" type="button" data-copy-var="${t.name}" title="Copiar var(${t.name})">${t.name}</button>${t.desc ? `<span class="tk-desc">${esc(t.desc)}</span>` : ''}</td><td class="tk-val" data-val="${t.name}"></td></tr>`).join('')}
      </tbody></table></section>`).join('');
    refreshCatalog(); filterCatalog();
    if (location.hash.startsWith('#tk-')) document.getElementById(location.hash.slice(1))?.scrollIntoView({ block: 'center' });
  }
  function refreshCatalog() {
    $$('#token-catalog [data-val]').forEach(td => { td.textContent = cssVar(td.dataset.val) || '—'; });
  }
  function filterCatalog() {
    const q = ($('#token-filter')?.value || '').trim().toLowerCase();
    let shown = 0;
    $$('#token-catalog [data-group]').forEach(sec => {
      let any = 0;
      $$('tr[data-token]', sec).forEach(tr => { const hit = !q || tr.dataset.search.includes(q); tr.hidden = !hit; if (hit) any++; });
      sec.hidden = !any; shown += any;
    });
    const c = $('#token-count'); if (c) c.textContent = q ? (shown ? `${shown} ${shown === 1 ? 'token encontrado' : 'tokens encontrados'}` : 'Nenhum token encontrado') : '';
  }
  $('#token-filter')?.addEventListener('input', filterCatalog);
  $('#token-catalog')?.addEventListener('click', (e) => { const b = e.target.closest('[data-copy-var]'); if (b) copy(`var(${b.dataset.copyVar})`, b.dataset.copyVar); });
  new MutationObserver(refreshCatalog).observe(root, { attributes: true, attributeFilter: ['data-theme'] });
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', refreshCatalog);

  /* ---------------- tokens.css ---------------- */
  const tokensPre = $('#tokens-source');
  let tokensText = '';
  if (tokensPre) {
  fetch('tokens.css').then(r => { if (!r.ok) throw 0; return r.text(); }).then(t => { tokensText = t; tokensPre.textContent = t; buildCatalog(t); })
    .catch(() => {
      try {
        const sheet = [...document.styleSheets].find(s => (s.href || '').endsWith('tokens.css'));
        tokensText = [...sheet.cssRules].map(r => r.cssText).join('\n\n');
        tokensPre.textContent = tokensText;
        buildCatalog(tokensText);
      } catch (e) { tokensPre.textContent = 'Não foi possível ler tokens.css a partir desta página. Abra o arquivo design-system/tokens.css diretamente.'; const h = $('#token-catalog'); if (h) h.innerHTML = '<p class="token-loading">O catálogo precisa de um servidor local: abra o design system por http (por exemplo, npx serve design-system), não direto do arquivo.</p>'; }
    });
  $('#copy-tokens').addEventListener('click', () => copy(tokensText || tokensPre.textContent, 'tokens.css'));
  }

  syncThemeButton();
  refreshThemeValues();
  if (location.hash) document.getElementById(location.hash.slice(1))?.scrollIntoView();
})();

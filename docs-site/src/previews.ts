// Telas do EpistulaMail mostradas na documentação (AppPreview): o HTML do design system, com as mesmas classes do
// app. Os textos são os da interface real, que hoje só existe em português; a legenda de cada tela é traduzida.
const x = '<span class="chip__remove"><svg class="i i--sm"><use href="#i-x"/></svg></span>';

export const previews = {
  search: `
<div style="display:grid;gap:8px;max-width:44rem">
  <div class="input input--search chip-input is-focus"><svg class="i"><use href="#i-search"/></svg>
    <span class="chip"><span class="chip__op">De</span><span class="avatar avatar--green">AR</span><span class="chip__name">Ana Ribeiro</span>${x}</span>
    <span class="chip chip--filter"><svg class="i"><use href="#i-clock"/></svg><span class="chip__name">Este ano</span>${x}</span>
    <span class="chip chip--filter"><svg class="i"><use href="#i-clip"/></svg><span class="chip__name">Com anexo</span>${x}</span>
    <input value="contrato de: ca" aria-label="Buscar"></div>
  <div class="menu menu--wide" style="position:static">
    <div class="menu__heading">Pessoas</div>
    <span class="menu-item is-selected"><span class="avatar avatar--sm avatar--magenta">CL</span><span class="menu-item__text">Carla Lima</span><span class="menu-item__meta">carla@lumen.com.br · 214</span></span>
    <span class="menu-item"><span class="avatar avatar--sm avatar--teal">CM</span><span class="menu-item__text">Caio Moreira</span><span class="menu-item__meta">caio@estudio.art · 9</span></span>
    <div class="menu__sep"></div>
    <span class="menu-item"><svg class="i"><use href="#i-search"/></svg><span class="menu-item__text">Buscar "ca" no remetente</span></span>
  </div>
</div>`,

  results: `
<div style="border:1px solid var(--color-border);border-radius:12px;overflow:hidden;background:var(--color-surface)">
  <div class="search-bar"><span class="search-bar__count">3 resultados</span>
    <div class="segmented"><span class="btn btn--compact" aria-checked="false">Esta conta</span><span class="btn btn--compact" aria-checked="true">Todas as contas</span></div>
    <div class="search-bar__end"><span class="btn btn--subtle btn--compact"><svg class="i"><use href="#i-plus"/></svg>Salvar busca</span></div></div>
  <div class="thread-list thread-list--standard">
    <div class="thread is-unread"><input type="checkbox" class="checkbox thread__check"><span class="star-btn thread__star" aria-pressed="false"><svg class="i"><use href="#i-star"/></svg></span><div class="thread__from"><span class="thread__from-name">Ana Ribeiro</span></div><div class="thread__meta"><span class="thread__date">10:42</span></div><div class="thread__subject"><span class="avatar avatar--xs avatar--purple thread__account">T</span><span class="label">Caixa de entrada</span><span class="thread__subject-text">Revisão do contrato: Fornecedor Lumen</span><span class="thread__icons"><svg class="i i--sm"><use href="#i-clip"/></svg></span></div></div>
    <div class="thread"><input type="checkbox" class="checkbox thread__check"><span class="star-btn thread__star" aria-pressed="true"><svg class="i"><use href="#i-star"/></svg></span><div class="thread__from"><span class="thread__from-name">Ana Ribeiro</span></div><div class="thread__meta"><span class="thread__date">2 set</span></div><div class="thread__subject"><span class="avatar avatar--xs thread__account">P</span><span class="label label--purple">Jurídico</span><span class="thread__subject-text">Contrato do apartamento assinado</span></div></div>
    <div class="thread"><input type="checkbox" class="checkbox thread__check"><span class="star-btn thread__star" aria-pressed="false"><svg class="i"><use href="#i-star"/></svg></span><div class="thread__from"><span class="thread__from-name">Ana Ribeiro</span></div><div class="thread__meta"><span class="thread__date">2015</span></div><div class="thread__subject"><span class="avatar avatar--xs avatar--archive thread__account"><svg class="i"><use href="#i-archive"/></svg></span><span class="label">Enviados</span><span class="thread__subject-text">Minuta do contrato antigo</span></div></div>
  </div>
</div>`,

  saved: `
<nav class="sidebar" style="max-width:16rem;border:1px solid var(--color-border);border-radius:12px">
  <div class="nav-group">
    <span class="nav-item"><svg class="i"><use href="#i-inbox"/></svg><span class="nav-item__text">Caixa de entrada</span><span class="nav-item__count">12</span></span>
    <span class="nav-item"><svg class="i"><use href="#i-send"/></svg><span class="nav-item__text">Enviados</span></span>
  </div>
  <div class="nav-group">
    <div class="nav-heading">Buscas salvas</div>
    <span class="nav-item has-unread" aria-current="page"><svg class="i"><use href="#i-search"/></svg><span class="nav-item__text">Contratos com a Ana</span><span class="nav-item__count">3</span></span>
    <span class="nav-item"><svg class="i"><use href="#i-search"/></svg><span class="nav-item__text">Notas fiscais deste ano</span></span>
  </div>
</nav>`,

  rules: `
<div class="rule-list" style="max-width:44rem;background:var(--color-surface);border-radius:12px">
  <div class="rule-row"><span class="rule-row__grip"><svg class="i"><use href="#i-grip"/></svg></span><input type="checkbox" class="toggle" checked><div class="rule-row__body"><span class="rule-row__name">Newsletters</span><span class="rule-sentence">Se <span class="rule-term">é de uma lista de e-mails</span><svg class="i i--sm"><use href="#i-arrow-right"/></svg><span class="rule-term rule-term--action"><svg class="i"><use href="#i-folder-move"/></svg>Mover para Leitura</span></span></div></div>
  <div class="rule-row"><span class="rule-row__grip"><svg class="i"><use href="#i-grip"/></svg></span><input type="checkbox" class="toggle" checked><div class="rule-row__body"><span class="rule-row__name">Recibos</span><span class="rule-sentence">Se <span class="rule-term">Assunto contém nota fiscal</span><svg class="i i--sm"><use href="#i-arrow-right"/></svg><span class="rule-term rule-term--action"><svg class="i"><use href="#i-tag"/></svg>Financeiro</span></span></div></div>
  <div class="rule-row"><span class="rule-row__grip"><svg class="i"><use href="#i-grip"/></svg></span><input type="checkbox" class="toggle"><div class="rule-row__body"><span class="rule-row__name">Loja</span><span class="rule-sentence">Se <span class="rule-term">De contém @loja.com</span><svg class="i i--sm"><use href="#i-arrow-right"/></svg><span class="rule-term rule-term--action"><svg class="i"><use href="#i-archive"/></svg>Arquivar</span></span></div></div>
</div>`,

  crypto: `
<div style="display:grid;gap:16px;max-width:44rem">
  <div class="message message--card" style="background:var(--color-surface);border-radius:12px;padding:16px"><div class="message__head"><span class="avatar avatar--lg avatar--green">AR</span><div style="min-width:0"><div class="message__from">Ana Ribeiro <span class="message__addr">&lt;ana@lumen.com.br&gt;</span></div><div class="sec-status"><span class="sec-badge"><svg class="i"><use href="#i-lock"/></svg>Criptografada · OpenPGP</span><span class="sec-badge"><svg class="i"><use href="#i-badge-check"/></svg>Assinatura verificada</span></div></div><span class="message__time">10:42</span></div></div>
  <div class="compose compose--inline" style="background:var(--color-surface);border-radius:12px">
    <div class="compose__fields"><div class="input input--line chip-input"><span class="input__prefix">Para</span><span class="chip"><span class="avatar avatar--teal">PH</span><span class="chip__name">Paulo Henrique</span><span class="chip__key"><svg class="i"><use href="#i-lock"/></svg></span></span></div></div>
    <div class="compose__security"><span class="btn btn--subtle btn--compact" aria-pressed="true"><svg class="i"><use href="#i-lock"/></svg>Criptografar</span><span class="btn btn--subtle btn--compact" aria-pressed="true"><svg class="i"><use href="#i-signature"/></svg>Assinar</span><span class="compose__security-tech">OpenPGP</span></div>
  </div>
</div>`,

  keys: `
<table class="key-table" style="max-width:44rem;background:var(--color-surface);border-radius:12px">
  <thead><tr><th>Nome e e-mail</th><th>ID</th><th>Situação</th></tr></thead>
  <tbody>
    <tr><td><span class="key-table__who"><b>Ana Ribeiro</b>ana@lumen.com.br</span></td><td class="key-table__id">5C21 08AA 91F3</td><td><span class="lozenge lozenge--success">Verificada</span></td></tr>
    <tr><td><span class="key-table__who"><b>Paulo Henrique</b>paulo@exemplo.org</span></td><td class="key-table__id">19FE 7A02 BB65</td><td><span class="lozenge lozenge--inprogress">Aceita</span> <span class="lozenge lozenge--new">Pós-quântica</span></td></tr>
    <tr><td><span class="key-table__who"><b>Davi Santos</b>davi@estudio.art</span></td><td class="key-table__id">E7D0 44B1 0C9A</td><td><span class="lozenge">Ainda não aceita</span></td></tr>
  </tbody>
</table>`,

  archive: `
<div class="modal" style="max-width:32rem;margin-inline:auto">
  <div class="modal__head"><h3 class="modal__title">Caixa de 2015 está pronta</h3></div>
  <div class="modal__body"><dl class="import-summary"><div><dt>Entraram</dt><dd>47.034</dd></div><div><dt>Repetidas</dt><dd>81</dd></div><div><dt>Falharam</dt><dd>0</dd></div></dl></div>
  <div class="modal__foot"><span class="btn btn--primary">Abrir o arquivo</span></div>
</div>`,

  accounts: `
<nav class="account-rail" style="border-radius:12px;background:var(--color-surface);display:inline-flex">
  <div class="account-rail__list" role="list">
    <div role="listitem"><span class="account-rail__item" aria-current="true"><span class="avatar avatar--lg avatar--purple">T</span><span class="badge badge--primary account-rail__badge">12</span><span class="account-rail__name">Trabalho</span></span></div>
    <div role="listitem"><span class="account-rail__item"><span class="avatar avatar--lg">P</span><span class="badge badge--primary account-rail__badge">4</span><span class="account-rail__name">Pessoal</span></span></div>
    <div role="listitem"><span class="account-rail__item"><span class="avatar avatar--lg avatar--orange">F</span><span class="account-rail__name">Família</span></span></div>
    <div role="listitem"><span class="account-rail__item"><span class="avatar avatar--lg avatar--archive"><svg class="i"><use href="#i-archive"/></svg></span><span class="account-rail__name">Arquivo</span></span></div>
  </div>
  <div class="account-rail__sep" role="separator"></div>
  <span class="account-rail__add"><svg class="i"><use href="#i-plus"/></svg></span>
</nav>`,

  conversation: `
<div class="thread-list" style="max-width:44rem;background:var(--color-surface);border-radius:12px;overflow:hidden">
  <div class="thread is-unread"><input type="checkbox" class="checkbox thread__check"><span class="star-btn thread__star" aria-pressed="false"><svg class="i"><use href="#i-star"/></svg></span><div class="thread__from"><span class="thread__from-name">Marina Couto, Você</span><span class="thread__count">3</span></div><div class="thread__meta"><span class="thread__date">10:42</span></div><div class="thread__subject"><span class="thread__subject-text">Revisão do contrato</span><span class="thread__icons"><svg class="i i--sm"><use href="#i-clip"/></svg></span></div><div class="thread__snippet">Segue a versão 3 com os ajustes da cláusula 7.2.</div></div>
  <div class="thread"><input type="checkbox" class="checkbox thread__check"><span class="star-btn thread__star" aria-pressed="true"><svg class="i"><use href="#i-star"/></svg></span><div class="thread__from"><span class="thread__from-name">Banco Atlântico</span></div><div class="thread__meta"><span class="thread__date">ontem</span></div><div class="thread__subject"><span class="label label--purple">Financeiro</span><span class="thread__subject-text">Seu extrato de outubro</span></div><div class="thread__snippet">O extrato está disponível no aplicativo.</div></div>
</div>`,

  undo: `
<div class="toast toast--info" style="max-width:24rem;margin-inline:auto"><svg class="i"><use href="#i-send"/></svg><div><p class="toast__title">Enviando em 5 s</p><p class="toast__desc">Para: Marina Couto</p><div class="toast__actions"><button type="button">Desfazer</button></div></div><div class="progress toast__timer"><span style="width:60%"></span></div></div>`,

  update: `
<div style="display:grid;gap:16px;justify-items:center">
  <div class="menu notice-panel" style="position:static;width:22rem"><div class="notice-panel__head"><h3 class="notice-panel__title">Notificações</h3></div>
    <div class="notice-panel__list" role="list"><span class="notice is-unread" role="listitem"><span class="notice__icon" aria-hidden="true"><svg class="i i--sm"><use href="#i-download"/></svg></span><span class="notice__body"><span class="notice__title">Nova versão do EpistulaMail: 1.0.3</span><span class="notice__text">Clique para ver o que mudou e atualizar.</span></span><span class="notice__time">09:15</span></span></div></div>
  <div class="modal" style="max-width:30rem"><div class="modal__head"><h3 class="modal__title">Atualizar o EpistulaMail para a versão 1.0.3?</h3></div>
    <div class="modal__body"><p style="margin:0">Você está na versão 1.0.2. O EpistulaMail fecha, instala a versão 1.0.3 e abre de novo. As contas, as mensagens e as configurações continuam.</p></div>
    <div class="modal__foot"><span class="btn btn--subtle">Agora não</span><span class="btn btn--primary">Atualizar e reiniciar</span></div></div>
</div>`,

  unlock: `
<div class="unlock" style="min-height:0;padding:24px"><div class="unlock__card">
  <img class="brand-logo" src="/favicon.png" alt="">
  <h2 class="unlock__title">O EpistulaMail está bloqueado</h2>
  <p class="unlock__text">Digite a senha do EpistulaMail para ver as suas mensagens.</p>
  <div class="unlock__form"><div class="field"><label class="field__label">Senha do EpistulaMail</label><div class="input input--password is-focus"><input type="password" value="frase-secreta"></div></div><span class="btn btn--primary btn--block">Desbloquear</span></div>
</div></div>`,
};

/** Legenda de cada tela, nos idiomas da documentação. */
export const captions: Record<keyof typeof previews, Record<string, string>> = {
  search: { "pt-BR": "A busca com chips e as sugestões de pessoas", en: "Search with chips and people suggestions", es: "La búsqueda con chips y las sugerencias de personas", fr: "La recherche avec des puces et les suggestions de personnes", it: "La ricerca con i chip e i suggerimenti di persone" },
  results: { "pt-BR": "Resultados em todas as contas, com o avatar de cada conta", en: "Results across all accounts, with each account's avatar", es: "Resultados en todas las cuentas, con el avatar de cada una", fr: "Résultats dans tous les comptes, avec l'avatar de chacun", it: "Risultati in tutti gli account, con l'avatar di ciascuno" },
  saved: { "pt-BR": "Buscas salvas na coluna de pastas", en: "Saved searches in the folder column", es: "Búsquedas guardadas en la columna de carpetas", fr: "Recherches enregistrées dans la colonne des dossiers", it: "Ricerche salvate nella colonna delle cartelle" },
  rules: { "pt-BR": "Regras lidas como frases, com a ordem e o liga e desliga", en: "Rules that read as sentences, with order and on/off switches", es: "Reglas que se leen como frases, con orden e interruptor", fr: "Des règles qui se lisent comme des phrases, avec l'ordre et l'interrupteur", it: "Regole che si leggono come frasi, con ordine e interruttore" },
  crypto: { "pt-BR": "Mensagem cifrada e verificada, e o composer com o cadeado ligado", en: "An encrypted, verified message and the composer with the lock on", es: "Un mensaje cifrado y verificado, y el editor con el candado activado", fr: "Un message chiffré et vérifié, et l'éditeur avec le cadenas activé", it: "Un messaggio cifrato e verificato, e l'editor con il lucchetto attivo" },
  keys: { "pt-BR": "O gerenciador de chaves OpenPGP", en: "The OpenPGP key manager", es: "El gestor de claves OpenPGP", fr: "Le gestionnaire de clés OpenPGP", it: "Il gestore delle chiavi OpenPGP" },
  archive: { "pt-BR": "O resumo no fim da importação", en: "The summary at the end of an import", es: "El resumen al final de la importación", fr: "Le résumé à la fin de l'importation", it: "Il riepilogo alla fine dell'importazione" },
  accounts: { "pt-BR": "A coluna de contas, com um arquivo local", en: "The account column, with a local archive", es: "La columna de cuentas, con un archivo local", fr: "La colonne des comptes, avec une archive locale", it: "La colonna degli account, con un archivio locale" },
  conversation: { "pt-BR": "A lista de conversas", en: "The conversation list", es: "La lista de conversaciones", fr: "La liste des conversations", it: "L'elenco delle conversazioni" },
  undo: { "pt-BR": "Desfazer o envio", en: "Undo send", es: "Deshacer el envío", fr: "Annuler l'envoi", it: "Annulla l'invio" },
  update: { "pt-BR": "O aviso no sino e a confirmação da atualização", en: "The bell notice and the update confirmation", es: "El aviso en la campana y la confirmación de la actualización", fr: "La notification dans la cloche et la confirmation de mise à jour", it: "L'avviso nella campanella e la conferma dell'aggiornamento" },
  unlock: { "pt-BR": "A tela de bloqueio", en: "The lock screen", es: "La pantalla de bloqueo", fr: "L'écran de verrouillage", it: "La schermata di blocco" },
};

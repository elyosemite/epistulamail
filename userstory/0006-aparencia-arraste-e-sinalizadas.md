## User Story 6: Aparência (tema e fundo), barra de rolagem, arrastar com contador e sinalizadas na coluna de pastas

**Como** usuário do Epistula,
**quero** escolher o tema (claro, escuro ou o do sistema) e a cor do fundo, ver barras de rolagem no estilo do app, ter um retorno claro ao arrastar várias conversas e encontrar as mensagens sinalizadas abaixo das pastas,
**para que** o app fique com a minha cara e eu organize a caixa mais rápido.

---

### 1. Contexto

Continuação da US5. Também corrige um defeito da US5: a coluna de conteúdo não rolava (o bloco cortava o conteúdo com `overflow: hidden`). A correção já está em `components.css` (`.app-shell--blocks .app-shell__reader { overflow: auto }`).

### 2. Regra do design system (obrigatória)

Vale a seção 2 da US1. As peças novas estão no rascunho `design-system/rascunho-us6.html`.

### 3. Lacunas no design system

| # | Lacuna | Proposta |
|---|---|---|
| L35 | Barra de rolagem | Global, nos dois eixos: 12px, trilho transparente, alça arredondada com `--color-scrollbar-thumb` e `--color-scrollbar-thumb-hovered` (novos tokens, claro e escuro) |
| L36 | Escolha de tema | `.theme-picker`: três cartões de rádio (Claro, Escuro, Sistema) com ícone, nome e descrição |
| L37 | Paleta do fundo | Tokens `--color-canvas-<cor>` (claro e escuro) e `.color-swatch__fill` (amostra quadrada da cor, no `.color-swatch` que já existe) |
| L38 | Arrastar conversas | `cursor: grab` nas linhas marcadas, `cursor: grabbing` durante o arraste (`.is-grabbing` no `<html>`), e o `.drag-ghost` com o cursor no canto superior esquerdo e o texto "Mover N conversas" |
| L39 | Sinalizadas na coluna de pastas | Grupo `.nav-group` "Sinalizadas" abaixo dos marcadores, com um `.nav-item` por prazo e `.nav-item.is-overdue` (ícone e contador em vermelho) |
| — | Ícone novo | `monitor` (opção Sistema) |

### 4. Critérios de aceite

**CA1. Rolagem da coluna de conteúdo**

- A leitura rola na vertical quando a mensagem é maior que o bloco.
- E-mails HTML mais largos que o bloco rolam na horizontal dentro do quadro da mensagem; nada da página rola na horizontal.

**CA2. Barra de rolagem**

- Todas as áreas roláveis do app (pastas, lista, leitura, menus, configurações) usam a barra do DS, nos dois eixos e nos dois temas.

**CA3. Tema (Configurações → Aparência)**

- Claro, Escuro e Sistema (segue o Windows e muda junto com ele). Padrão: Sistema.
- Vale na hora e fica salvo neste computador.

**CA4. Cor do fundo (Configurações → Aparência)**

- 8 cores: Padrão, Azul, Verde-azulado, Verde, Roxo, Magenta, Laranja e Areia. Cada cor tem uma versão clara e uma escura, e segue o tema.
- Vale na hora e fica salva neste computador.

**CA5. Arrastar conversas**

- Passar o mouse sobre uma linha marcada mostra a mão aberta (`grab`).
- Ao arrastar, o cursor vira a mão fechada (`grabbing`) e um rótulo acompanha o cursor, com o cursor no canto superior esquerdo: "Mover 1 conversa", "Mover 3 conversas" (D1).
- A pasta sob o cursor fica destacada como na US5. Soltar numa pasta move; soltar fora ou `Esc` cancela.
- O arraste passa a ser feito pelo app (eventos de ponteiro), e não pelo arrastar e soltar do sistema, porque no arrastar do sistema o navegador não deixa trocar o cursor.

**CA6. Sinalizadas abaixo das pastas (D2)**

- Se a conta tiver mensagens sinalizadas e não concluídas, aparece o grupo "Sinalizadas" abaixo dos marcadores, com um item por prazo que tenha mensagens: Atrasadas, Hoje, Esta semana, Mais tarde e Sem data, cada um com a quantidade.
- Clicar num item mostra essas mensagens na lista, com a pasta de cada uma (como na busca).
- Sem nenhuma sinalizada, o grupo não aparece.

### 5. Backend

- `app_setting` ganha `theme` (`light`, `dark`, `system`) e `canvas` (nome da cor). Comandos `get_appearance` e `set_appearance`.
- `flagged_counts { accountId }` → quantidades por prazo; `list_flagged { accountId, bucket }` → mensagens (até 200), consulta só no banco local.

### 6. Decisões

- **D1 (aprovado em 24 out 2026):** o texto do rótulo em português, "Mover 1 conversa", como o resto do app. O pedido escreveu "Move 1 conversation".
- **D2 (aprovado):** "lista de flags" = as mensagens sinalizadas da US4, agrupadas por prazo. Se a ideia era outra (por exemplo, os marcadores do Gmail ou cada mensagem sinalizada listada na coluna), ajusto no rascunho.
- **D3 (aprovado):** tema e fundo ficam salvos no banco local (valem para todas as contas deste computador).

### 7. Implementação (24 out 2026)

**Design system** (rascunho aprovado, incorporado e apagado):

- `tokens.css`: `--color-scrollbar-thumb`, `--color-scrollbar-thumb-hovered` e `--color-canvas-blue|teal|green|purple|magenta|orange|sand`, todos com valores claro e escuro.
- `components.css`: barra de rolagem global, `.theme-picker`/`.theme-option`, `.color-swatch__fill`, `.thread.is-checked` com `grab`, `html.is-grabbing`, `.drag-ghost--follow` e `.nav-item.is-overdue`. A correção da rolagem da leitura (`.app-shell--blocks .app-shell__reader { overflow: auto }`) também ficou no DS.
- Páginas: `selecao.html` (tema e cor do fundo), `espacamento.html` (barra de rolagem), `threads.html` (arrastar), `sidebar.html` (sinalizadas e rótulo novo) e `modais.html` (seção Aparência).
- Ícone `monitor`.

**Código**

- Aparência: `lib/appearance.ts` aplica o tema (`data-theme` no `<html>`; Sistema o remove) e o fundo (`--app-background`). O banco (`app_setting`: `theme`, `canvas`) é a fonte; uma cópia fica no armazenamento local só para a próxima abertura pintar certo antes da resposta do banco. Primitivas `ui/ThemePicker` e `ui/CanvasPicker`; seção `features/settings/AppearanceSettings`.
- Arrastar: `layout/EmailList` usa eventos de ponteiro (limiar de 5px), procura a pasta sob o ponteiro por `[data-drop-folder]` e mostra o `DragGhost` ("Mover N conversas"). `Esc` ou soltar fora cancela; o clique depois do arraste não abre a mensagem. O arrastar nativo (HTML5) saiu.
- Sinalizadas: `layout/FlaggedItem`, grupo em `layout/FoldersColumn`, `lib/flagged.ts` (datas locais, prazos visíveis). O `Mailbox` trata busca e sinalizadas como listas de várias pastas (`crossFolder`), e as quantidades acompanham sinalizar, estrela, mover e a sincronização.
- Backend: `flagged_counts`, `list_flagged`, `get_appearance` e `set_appearance`; a barra de rolagem do quadro do e-mail ganhou as cores do tema claro (`sanitize::email_document`).

**Verificado**

- **Rust: 86 testes** (2 novos: sinalizadas por prazo, sem concluídas, sem estrela, sem outra conta e contando uma vez a mensagem em vários marcadores; aparência com padrão e valores inválidos recusados), estáveis em 3 execuções; `clippy` sem avisos.
- **Frontend: 49 testes** (4 novos: datas de referência inclusive no fim de semana, prazos visíveis, validação e aplicação da aparência); `tsc` e build ok.
- **Interface com backend simulado:** Aparência nos dois temas com fundo roxo escuro, sinalizadas com Atrasadas em vermelho e a lista do prazo com a pasta de cada mensagem, arraste de 3 conversas até "Projetos" com o rótulo e a pasta destacada, e a leitura rolando com a barra do DS.

**Pendente**

- Teste manual no app: trocar tema e fundo e reabrir; arrastar com o mouse de verdade (cursor de mão); sinalizar mensagens e ver o grupo "Sinalizadas".

### 8. Correção: abertura lenta (24 out 2026)

- **Medição:** com logs de tempo desde o início do processo, o backend fica pronto em 10–20 ms e responde `list_accounts` em 3 ms, mas na primeira abertura depois de iniciar o `pnpm tauri dev` a tela só pedia as contas depois de **218 s**. Sem o Tauri, a primeira carga da página pelo Vite levou **247 s** (todas as requisições lentas ao mesmo tempo, inclusive `/@vite/client`).
- **Causa:** o observador de arquivos do Vite varria `.tools/` (o Strawberry Perl portátil do build do OpenSSL, da US3: 20.858 arquivos em 2.166 pastas). Só afeta o desenvolvimento; o app instalado não usa o Vite.
- **Correção:** `vite.config.ts` ignora `.tools/` e `dist/` (além de `src-tauri/`). Primeira carga medida: 7 s com o navegador de teste; as seguintes, cerca de 2 s; no app, primeira tela em 1,4 s depois do processo abrir.
- **Carregamento nos componentes:** `layout/AppSkeleton` mostra a casca em blocos com cada componente em skeleton (contas, busca, comandos, pastas, lista e leitura), no estado "carregando" do App. O `index.html` traz a mesma marcação, gerada a partir do componente, para aparecer antes do JavaScript, sem salto quando o React assume.
- **Diagnóstico mantido:** o log "primeira tela desenhada" (ms desde o início do processo) e o aviso "acesso lento ao banco" (espera ou consulta acima de 100 ms, só tempos).

### 9. Tema escuro mais preto (24 out 2026)

Rascunho `rascunho-dark.html` aprovado, aplicado e apagado.

- **Superfícies:** quase pretas, sem `#000` puro: fundo da janela `#030304`, blocos `#0E1013`, rebaixada `#090A0C`, elevada `#15181C`, menus e modais `#1B1F24`. As 7 cores do fundo da US6 também escureceram, no mesmo tom.
- **Texto:** `#E6EAF0`, secundário `#BCC5D0` e terciário `#9AA5B2`; marca e links `#75AEFF`, botão principal `#6AA7FF`.
- **Contraste medido (WCAG):** todo texto passa de 7:1 nas superfícies (texto 15,8:1, terciário 7,6:1, azul 8,4:1, selecionado 8,0:1, botão principal 7,8:1). Antes, cinco pares ficavam entre 4,5 e 7:1.
- **Separação:** no tema escuro, `--shadow-raised` e `--shadow-overlay` ganharam um contorno de 1px em branco translúcido, porque a sombra não aparece no preto.
- **Onde:** 53 tokens nos dois blocos escuros do `tokens.css` (o que segue o sistema e o escolhido manualmente). O tema claro não mudou. A página `cores.html` explica os valores e o contraste.


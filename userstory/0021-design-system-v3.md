## User Story 21: Design system v3, arquitetura e governança

**Como** pessoa que mantém o EpistulaMail,
**quero** que o design system siga a arquitetura e a governança do `EpistulaMailDesignSystem-v3.md`,
**para que** fique claro onde cada peça mora, em que fase ela está, como entra e se o sistema está saudável.

---

### 1. Contexto

O v3 define a arquitetura do design system: camadas, tokens em três níveis, temas, acessibilidade, conteúdo, padrões, componentes de produto, documentação, governança e métricas. Esta entrega aplica o documento inteiro ao design system (`design-system/`), do inventário à governança contínua, na ordem de adoção do §14.

Mudar o app para usar o que foi definido (por exemplo, os novos selos de segurança) não faz parte desta entrega; está em `design-system/ROADMAP.md`. As peças do app só mudaram nos comentários `// DS:`, que apontam para as páginas novas.

### 2. Critérios de aceite

**CA1. Camadas e registro (§2, §3)**

- O índice agrupa as páginas em Fundamentos, Core, Padrões, Produto e Layouts e páginas. Cada página tem um status (rascunho, experimental, beta, estável, descontinuado, removido), mostrado como palavra no índice e ao lado do título.
- A página "Sidebar" foi separada em `navegacao.html` (Core), `contas.html` e `pastas.html` (Produto). "Avatar, badge e lozenge" virou três páginas.
- `arquitetura.html` descreve as camadas, a regra de classificação, a regra dos três usos, o mapa de primitives e utilities (medido no código) e a ordem de adoção. `layouts.html` descreve as regiões de cada tela.

**CA2. Tokens (§4)**

- Três níveis: 117 primitivos `--palette-*` (só `tokens.css` os usa), os semânticos e os de componente (locais, em `components.css`).
- Domínios novos: foco, opacidade, ícones, breakpoints e densidade nomeada. Os valores soltos de `components.css` que coincidiam com eles passaram a usar o token.
- Nenhum valor mudou: os 178 tokens que já existiam resolvem para o mesmo valor nos dois temas.
- `variaveis.html` é um catálogo montado de `tokens.css`, por grupo, com prévia, valor do tema ativo e filtro.

**CA3. Temas e densidade (§5)**

- Claro, escuro e alto contraste (`prefers-contrast: more` e `data-contrast="more"`), que só reaponta tokens. `temas.html` mostra o efeito ao vivo, a densidade e a regra da cor de destaque da conta.

**CA4. Acessibilidade, conteúdo e tradução (§6)**

- `acessibilidade.html` ganhou a tabela dos requisitos mínimos da WCAG 2.2 AA e onde cada um é cumprido.
- `conteudo.html` traz o tom de voz, o microtexto, as regras de tradução e uma demonstração de RTL. `components.css` usa propriedades lógicas; as exceções têm o comentário "físico".

**CA5. Padrões (§2.5)**

- Páginas de validação de formulário, confirmação, busca e seleção em lote, no modelo de padrão (problema, contexto, composição, fluxo, variações, estados, acessibilidade, conteúdo, anti-patterns). Vazios e carregamento ganharam o mesmo modelo.

**CA6. Produto e segurança (§2.6)**

- Seis estados de segurança com texto, ícone e forma próprios (L85 a L87), e o resumo de uma mensagem com cada conceito na sua linha. Verde só para a assinatura verificada.

**CA7. Documentação por componente (§7)**

- Cada página de componente termina com a ficha: camada, status, onde é usado (lido de `src/`), fazer e não fazer, tokens usados (lidos das regras de `components.css`), texto e tradução, testes, plataforma, relacionados e histórico.

**CA8. Governança (§8 a §13, §15)**

- `governanca.html`: maturidade e descontinuação, tokens, gates e definição de pronto, os quinze anti-patterns, a linha de base das métricas e os documentos.
- `CONTRIBUTING.md`, `DECISIONS.md` e `ROADMAP.md`, em `design-system/`.
- `pnpm ds:check` confere tokens, paridade do tema escuro, uso da paleta, breakpoints, registro de páginas, referências do app e tokens sem uso. Roda no CI.

### 3. Peças

- **L82** selo de maturidade. **L83** cartão do índice com selo. **L84** tabela dos níveis de token.
- **L85** selo de segurança neutro. **L86** selo tracejado (não verificado) e pontilhado (desconhecido). **L87** resumo de segurança.

### 4. Decisões

Registradas em `design-system/DECISIONS.md` (0003 a 0013). As desta história:

- **D1.** `arquivos` e `criptografia` começam em Beta, como os quatro padrões novos; as demais páginas, em Estável. Os status de maturidade são uma estimativa, e a decisão final é de quem mantém.
- **D2.** `vazios` e `carregamento` ficam em Padrões; `busca`, `regras` e `destinatarios`, em Produto.
- **D3.** Os rascunhos desta entrega foram aprovados e apagados.
- **D4.** Os estilos só das páginas do DS (`.maturity`, `.ov-card__head`, `.tier-table`, `.gates`, `.classes`, `.tk-*`, `.ficha__*`) ficam em `docs.css`.
- **D5.** Os avatares de exemplo usam as fotos CC0 de `design-system/images/people/`, as mesmas do site. O seletor de cor da conta fica sem foto, porque ali a cor é a informação.
- **D6.** O texto das páginas do DS não cita classes CSS; os nomes ficam numa linha "Classes" no fim de cada seção.
- **D7.** O catálogo de variáveis precisa de servidor local (http); aberto direto do arquivo, avisa.

### 5. Verificação

- Os 178 tokens originais, nos dois temas, resolvem para o mesmo valor depois da paleta, dos domínios novos e das propriedades lógicas.
- Os estilos computados e a posição de cada elemento das páginas do DS, comparados com a versão anterior, são idênticos, exceto a oscilação de `margin: auto` em alguns modais, que muda de lado a cada execução.
- O espelhamento foi conferido nos componentes, com a demonstração RTL.

### 6. Fora desta entrega

Em `design-system/ROADMAP.md`: aplicar os estados de segurança no app, extrair o helper de classes e os primitives de Esc e de setas, teste automático de acessibilidade e de regressão visual, tokens sem uso, valores fixos, tradução do app, validação do alto contraste e do RTL num ambiente real.

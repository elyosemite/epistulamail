# EpistulaMail Design System — v3

**Status:** Proposed  
**Version:** 3.0.0  
**Purpose:** Definir a arquitetura, os princípios, as camadas, os padrões de documentação e a governança do Design System do EpistulaMail.  
**Escopo:** Aplicação desktop de e-mail, com foco em consistência visual, acessibilidade, segurança perceptível, eficiência operacional e evolução sustentável.

> Este documento define a arquitetura do Design System. O fluxo operacional detalhado de contribuição deve ficar em `CONTRIBUTING.md`, e o planejamento de implementação em `ROADMAP.md`. Os nomes de componentes listados aqui são um catálogo candidato, não uma obrigação de implementar tudo de uma vez.

## 1. Princípios

### Separação explícita de responsabilidades

O sistema é dividido em Foundations, Primitives, Utilities, Core Components, Patterns, Product Components e Layouts & Pages. Cada elemento deve pertencer à camada mais baixa que consiga expressar sua responsabilidade sem incorporar regras de negócio ou decisões de uma tela específica.

### Construir para o produto, não para completar um catálogo

O Design System existe para resolver problemas recorrentes do EpistulaMail, não para reproduzir integralmente um UI kit. Componentes genéricos só entram no Core quando houver necessidade comprovada, reutilização real ou quando forem fundações necessárias para acessibilidade e composição.

**Regra dos três usos:** um componente genérico normalmente deve ter três casos de uso reais antes de ser promovido a Core. Exceções são permitidas para fundações, acessibilidade, segurança ou componentes essenciais da plataforma. A exceção deve ser registrada na decisão de arquitetura.

### Acessibilidade e segurança fazem parte do comportamento

Acessibilidade, navegação por teclado, suporte a leitores de tela, redução de movimento, estados de foco e mensagens claras não são acabamentos opcionais. Em um cliente de e-mail que prioriza privacidade, estados de criptografia, identidade, sessão e confiança também precisam ser comunicados de forma inequívoca, sem sugerir garantias que o sistema não consegue comprovar.

### Tokens antes de estilos locais

Cores, espaçamento, tipografia, densidade, bordas, elevação, movimento e foco devem vir de tokens. Valores fixos são aceitáveis apenas quando forem inerentes à implementação ou documentados como exceção. Nenhum componente deve introduzir cores ou dimensões arbitrárias sem justificativa.

### Documentação acompanha a implementação

Um componente não está concluído apenas porque funciona visualmente. A documentação, os estados, os testes, a acessibilidade e a compatibilidade com temas fazem parte da entrega.

## 2. Arquitetura em camadas

```text
EpistulaMail Design System
├── Foundations
│   ├── Design Tokens
│   ├── Theming
│   ├── Accessibility
│   ├── Content & Localization
│   ├── Iconography
│   └── Layout Principles
├── Primitives
│   ├── Focus & Keyboard
│   ├── Layering & Dismissal
│   ├── Composition
│   └── Positioning & Presence
├── Utilities
│   ├── Class & Style Utilities
│   ├── Refs & Composition Helpers
│   ├── Shared Hooks
│   └── Platform Utilities
├── Core Components
│   ├── Actions
│   ├── Forms & Inputs
│   ├── Feedback
│   ├── Data Display
│   ├── Navigation
│   ├── Overlays
│   └── Layout & Structure
├── Patterns
│   ├── Form Patterns
│   ├── Search Patterns
│   ├── Confirmation Patterns
│   ├── Loading & Empty States
│   └── Bulk Selection Patterns
├── Product Components
│   ├── Account Rail & Account Switcher
│   ├── Folder Tree & Mail Navigation
│   ├── Mail List & Search Results
│   ├── Thread & Message Reader
│   ├── Composer & Recipient Input
│   ├── Attachments & Signatures
│   ├── Security & Identity Indicators
│   └── Mail-Specific Actions
└── Layouts & Pages
    ├── Application Shell
    ├── Inbox Layout
    ├── Search Layout
    ├── Settings Layout
    └── Account & Security Setup
```

### 2.1 Foundations

Foundations estabelecem as regras e os valores compartilhados. Não contêm CSS específico de um componente, classes de página nem lógica de negócio.

Incluem tokens, temas, tipografia, acessibilidade, conteúdo, localização, iconografia e princípios de layout. Os tokens são uma parte central de Foundations, e não uma subpasta isolada que funcione como catálogo sem semântica.

### 2.2 Primitives

Primitives são mecanismos de baixo nível, preferencialmente headless, que viabilizam composição, foco, teclado e camadas de interface. Não definem o estilo visual final nem a semântica de negócio do e-mail.

Candidatos, conforme a necessidade real:
- `Portal`: renderização de conteúdo em uma camada apropriada.
- `Slot`: composição de elementos sem impor uma estrutura visual.
- `FocusScope`: contenção, restauração e gerenciamento de foco.
- `DismissableLayer`: fechamento por Escape ou interação externa.
- `RovingFocus`: navegação por teclado em grupos de itens.
- `VisuallyHidden`: conteúdo disponível para tecnologias assistivas.
- `Presence`: ciclo de presença e saída de elementos.
- `Popper` ou mecanismo equivalente: posicionamento de conteúdo flutuante.
- `Collection`: gestão de coleções ordenadas, se exigida pelos componentes.

Essa lista não determina que todos os mecanismos devam ser implementados internamente. Sempre que possível, reutilizar uma biblioteca confiável e compatível com a stack. Não criar uma abstração headless própria sem necessidade demonstrada.

### 2.3 Utilities

Utilities concentram lógica transversal que não é, por si só, um componente visual. Exemplos: combinação de classes (`cn`), composição de refs (`composeRefs`), hooks de preferência por movimento reduzido, detecção de plataforma e helpers compartilhados de estilo.

Uma utility não deve esconder comportamento de negócio, estado global inesperado ou efeitos colaterais não documentados. Não duplicar helpers equivalentes em componentes diferentes.

### 2.4 Core Components

Core contém componentes genéricos, reutilizáveis e sem conhecimento específico do domínio de e-mail. Cada componente deve ter uma responsabilidade clara, uma API estável e estados documentados.

**Catálogo candidato, sujeito à regra dos três usos:**

| Categoria | Componentes candidatos |
|---|---|
| Actions | `Button`, `IconButton`, `ButtonGroup`, `Toggle`, `ToggleGroup` |
| Forms & Inputs | `Input`, `Textarea`, `Select`, `Combobox`, `Checkbox`, `RadioGroup`, `Switch`, `Label`, `Field`, `InputGroup` |
| Feedback | `Alert`, `Toast`, `Progress`, `Skeleton`, `Spinner`, `EmptyState`, `InlineMessage`, `StatusDot` |
| Data Display | `Avatar`, `Card`, `Table`, `DataTable`, `List`, `ListItem`, `Separator`, `Attachment` quando comprovadamente genérico |
| Navigation | `Breadcrumb`, `Tabs`, `Pagination`, `NavItem`, `Sidebar` como estrutura genérica |
| Overlays | `Dialog`, `AlertDialog`, `Sheet`, `Drawer`, `Popover`, `ContextMenu`, `DropdownMenu`, `Tooltip`, `CommandPalette` |
| Layout & Structure | `Accordion`, `Collapsible`, `ResizablePanel`, `ScrollArea` |

A lista é deliberadamente menor que um catálogo de UI kit. `Carousel`, `Rating`, `Slider`, `AspectRatio`, `Menubar`, `NavigationMenu`, `InputOTP` e outros itens não devem ser adicionados apenas porque existem em uma biblioteca. Cada inclusão exige um caso de uso real ou uma justificativa de fundação.

Correções de classificação:
- `Command` ou `CommandPalette` pertence a Overlays, quando funcionar como paleta de comandos.
- `DatePicker` pertence a Forms & Inputs; `Calendar` pode pertencer a Data Display se existir um caso de uso de calendário.
- `Badge` tem uma única classificação canônica em Data Display / Status. Feedback deve usar `InlineMessage`, `StatusDot` ou outro componente apropriado, sem duplicar `Badge`.
- Estilos de texto pertencem a Foundations / Typography. Um componente `Text` só deve existir no Core se houver necessidade real de abstração.
- `Tooltip` pode ser classificado em Overlays, pois depende de posicionamento e comportamento de camada.

### 2.5 Patterns

Patterns são composições documentadas de dois ou mais componentes Core que resolvem uma tarefa recorrente, sem incorporar regras exclusivas de uma tela ou do domínio de e-mail.

Patterns iniciais candidatos:
- Form Validation Pattern: campos, validação, mensagens e resumo de erros.
- Search Pattern: entrada, filtros, sugestões, resultados e estado sem resultados.
- Confirmation Pattern: confirmação de ações destrutivas ou irreversíveis.
- Loading Pattern: carregamento inicial, atualização parcial e operações em andamento.
- Empty / Error Pattern: ausência de dados, falha recuperável e ação de recuperação.
- Bulk Selection Pattern: seleção de itens, contador, ações em lote e limpeza da seleção.

Cada Pattern deve explicar o problema resolvido, a composição recomendada, as variações permitidas e quando não deve ser usado. Um Pattern não deve virar um componente genérico disfarçado nem depender de uma página específica.

### 2.6 Product Components

Product Components representam conceitos reais do EpistulaMail. Podem compor Foundations, Primitives, Utilities, Core e Patterns, mas não devem ser promovidos ao Core enquanto carregarem semântica de e-mail.

Catálogo inicial candidato:
- `AccountRail` e `AccountSwitcher`
- `FolderTree` e `FolderTreeItem`
- `MailListItem` e `MailList`
- `Thread` e `MessageReader`
- `Composer`
- `RecipientInput`
- `AttachmentList` e `AttachmentItem`
- `SearchResults`
- `BulkMailActions`
- `RuleBuilder`
- `SignatureEditor`
- `DraftIndicator`
- `SnoozeAction`
- `ContactCard`
- `ShortcutsOverlay`
- `EncryptionStatus`
- `SignatureVerificationStatus`
- `IdentityTrustIndicator`
- `SessionSecurityNotice`

Esses nomes são candidatos, não uma promessa de implementação imediata. Priorizar a partir dos fluxos reais do produto. Componentes de segurança devem diferenciar claramente estados como “criptografado em trânsito”, “armazenado localmente com proteção”, “criptografia ponta a ponta ativa”, “assinatura válida”, “identidade não verificada” e “estado desconhecido”. Não usar um único indicador verde para conceitos de segurança diferentes.

### 2.7 Layouts & Pages

Layouts organizam regiões da aplicação; Pages representam telas completas e seus fluxos. Essa camada pode compor componentes de todas as anteriores, mas não deve introduzir novos tokens globais ou redefinir estilos de Core localmente.

Exemplos: `ApplicationShell`, layout de Inbox, layout de busca, páginas de configurações e fluxo de configuração de conta. A composição específica de uma página permanece na página até que exista um padrão recorrente que justifique extraí-la.

## 3. Regras de classificação

| Camada | Pode conter | Não deve conter |
|---|---|---|
| Foundations | Tokens, temas, regras de acessibilidade, tipografia e conteúdo | CSS específico de componente, classes de página, lógica de negócio |
| Primitives | Foco, teclado, posicionamento, composição e camadas headless | Estilo visual final, semântica de e-mail |
| Utilities | Hooks e helpers transversais pequenos e reutilizáveis | Componentes visuais ou lógica de domínio |
| Core | Componentes genéricos com API e estados definidos | Regras específicas do EpistulaMail ou layout de página |
| Patterns | Composições reutilizáveis de componentes Core | Componentes genéricos novos sem avaliação ou composição exclusiva de uma tela |
| Product | Conceitos e fluxos específicos do cliente de e-mail | Tokens globais e abstrações genéricas disfarçadas |
| Layouts & Pages | Estrutura de telas e composição de fluxos | Redefinições locais arbitrárias de Foundations ou Core |

### Exemplo: navegação lateral e contas

A expressão visual “ADS · Side Navigation”, seletores como `.sidebar` e `.nav-item`, e a descrição de uma coluna de contas não devem ficar misturados em uma única especificação.

A separação correta é:
1. **Foundations:** tokens de cor, tipografia, espaçamento, borda, foco e densidade usados pela navegação.
2. **Core:** `Sidebar`, `NavItem`, `Avatar`, `Tooltip` e `StatusDot`, se forem abstrações genéricas necessárias.
3. **Product:** `AccountRail`, que organiza contas no contexto do EpistulaMail.
4. **Product:** `SideNavigation`, se combinar navegação de e-mail, `AccountRail` e ações específicas do produto.
5. **Layouts & Pages:** a posição e a composição dessas regiões no `ApplicationShell`.

A regra de comportamento “uma conta por item, na ordem escolhida pelo usuário” pertence à documentação de `AccountRail`. A disposição da rail dentro da janela pertence ao layout.

## 4. Design Tokens

### 4.1 Três tiers

Os tokens devem ter três níveis com responsabilidades explícitas:

1. **Primitive tokens:** valores brutos, sem intenção de uso. Exemplos: `blue-600`, `gray-100`, `space-4`.
2. **Semantic tokens:** significado independente do componente. Exemplos: `text-primary`, `surface-default`, `border-subtle`, `focus-ring`.
3. **Component tokens:** ajustes específicos de um componente, criados apenas quando tokens semânticos não forem suficientes. Exemplos: `button-height`, `dialog-shadow`.

A maioria dos componentes deve depender de tokens semânticos. Component tokens não devem virar uma segunda paleta paralela nem ser criados para cada pequena diferença visual.

### 4.2 Domínios mínimos de tokens

- Color: superfícies, texto, bordas, estados, seleção, links e foco.
- Typography: família, escala, peso, altura de linha e espaçamento entre letras.
- Spacing: escala de espaços e regras de composição.
- Sizing: dimensões de controles, ícones e áreas interativas.
- Radius: raios de borda.
- Elevation: sombras e níveis de superfície.
- Border: largura e estilo de borda.
- Motion: duração, easing e transições.
- Z-index: níveis de sobreposição com ordem documentada.
- Opacity: níveis de opacidade autorizados.
- Density: confortável, compacta e, se necessário, outras densidades explicitamente suportadas.
- Focus: largura, offset e cor do indicador de foco.
- Breakpoints e container queries: regras responsivas baseadas na área útil, não apenas na largura da janela.
- Iconography: escala e regras de alinhamento dos ícones.

Não é necessário definir todos os valores de todos os domínios no primeiro release. É necessário definir a nomenclatura, a fonte de verdade e o processo para adicioná-los.

### 4.3 Pipeline e fonte de verdade

Quando houver biblioteca de design, o fluxo recomendado é:

`Figma Variables → validação de tokens → Style Dictionary (ou ferramenta equivalente) → CSS Custom Properties + tipos/constantes TypeScript → testes de consistência`

O pipeline deve validar nomes, referências semânticas, temas suportados e tokens órfãos. Se o projeto não usar Figma ou Style Dictionary, documentar a fonte de verdade efetiva e manter a mesma separação entre tiers.

Exemplo conceitual:

```css
:root {
  --color-blue-600: #2457c5;
  --color-text-primary: var(--color-gray-950);
  --color-surface-default: var(--color-white);
  --focus-ring: var(--color-blue-600);
}

[data-theme="dark"] {
  --color-text-primary: var(--color-gray-50);
  --color-surface-default: var(--color-gray-950);
}
```

Os valores acima são ilustrativos, não uma paleta oficial do EpistulaMail. A paleta real deve ser definida e validada separadamente.

## 5. Theming e densidade

O sistema deve prever desde o início:
- Tema claro.
- Tema escuro.
- Modo de alto contraste ou compatibilidade com as preferências de contraste do sistema operacional.
- Cor de destaque configurável por conta, quando aplicável ao produto.
- Densidade confortável e compacta, se a pesquisa de uso justificar ambas.

A cor de destaque por conta deve ser aplicada com escopo explícito e sem alterar inadvertidamente cores semânticas de sucesso, erro, aviso ou segurança. O tema global e a identidade visual de cada conta são conceitos distintos.

Todo componente deve ser verificado nos temas suportados. Contraste, foco, seleção, estados desabilitados e conteúdo truncado devem continuar compreensíveis. Não inferir o estado de segurança exclusivamente por cor.

## 6. Acessibilidade, localização e conteúdo

### 6.1 Acessibilidade

O objetivo é conformidade com WCAG 2.2 AA, sujeita a avaliação contínua. Requisitos mínimos:
- Fluxos críticos operáveis por teclado.
- Ordem e visibilidade de foco previsíveis.
- Nomes acessíveis para controles e ações por ícone.
- Labels, descrições e erros associados programaticamente aos campos.
- Gerenciamento de foco em diálogos, menus e overlays.
- Suporte a leitores de tela e semântica correta.
- Respeito à preferência por movimento reduzido.
- Contraste validado em cada tema.
- Alvos interativos com tamanho adequado à plataforma.
- Nenhuma informação essencial transmitida apenas por cor, posição ou ícone.

Testes automatizados ajudam, mas não substituem testes manuais com teclado e tecnologias assistivas.

### 6.2 i18n e RTL

Todo texto de interface deve ser externalizável e preparado para tradução. Evitar strings concatenadas que impeçam tradução, pressupostos sobre pluralização, datas, horários, números ou endereços de e-mail.

Layouts devem preferir propriedades lógicas, como `margin-inline-start`, em vez de assumir esquerda e direita. RTL deve ser tratado como requisito arquitetural; a validação completa pode ser escalonada conforme os idiomas suportados, mas a estrutura não deve bloquear sua adoção.

### 6.3 Voice & Tone

A interface deve ser clara, precisa e calma. Evitar alarmismo, promessas absolutas de segurança e mensagens vagas. Para erros, explicar o que aconteceu, o impacto provável e a ação disponível. Para operações sensíveis, descrever consequências antes da confirmação.

## 7. Documentação de componentes

Cada componente Core ou Product deve ter documentação adequada à sua complexidade. O template canônico contém:

1. **Overview:** definição, problema resolvido e quando usar.
2. **Status e maturidade:** Draft, Experimental, Beta, Stable, Deprecated ou Removed.
3. **Anatomy:** partes nomeadas e relações entre elas.
4. **API:** props, tipos, valores padrão, eventos e restrições.
5. **Variants e sizes:** variantes realmente suportadas.
6. **States:** default, hover, focus, active, disabled, loading, error, empty e outros estados pertinentes.
7. **Tokens:** tokens semânticos e component tokens utilizados.
8. **Usage guidelines:** recomendações e decisões de uso.
9. **Do / Don't:** exemplos corretos e anti-patterns específicos.
10. **Accessibility:** semântica, teclado, foco, ARIA e leitor de tela.
11. **Content & localization:** microcopy, truncamento, pluralização, i18n e RTL.
12. **Examples:** exemplos mínimos e casos avançados.
13. **Tests:** testes unitários, acessibilidade e regressão visual relevantes.
14. **Platform notes:** diferenças de comportamento entre plataformas, quando existirem.
15. **Related components:** alternativas e componentes relacionados.
16. **Changelog:** alterações relevantes, migrações e deprecações.

Nem todo componente precisa de uma longa explicação para cada seção; quando uma seção não se aplicar, registrar “não aplicável” com justificativa, em vez de omiti-la silenciosamente.

Patterns devem usar um template próprio: problema, contexto, composição, fluxo, variações, estados, acessibilidade, conteúdo, anti-patterns e exemplos.

## 8. Anti-patterns

1. Criar componentes para reproduzir todo o catálogo de uma biblioteca externa sem uso real.
2. Colocar regras de negócio do EpistulaMail em componentes Core.
3. Duplicar o mesmo componente em categorias diferentes.
4. Usar cores, espaçamentos ou tamanhos arbitrários sem token ou justificativa.
5. Criar tokens específicos para cada componente quando um token semântico existente resolve o caso.
6. Construir primitivas headless próprias sem demonstrar uma lacuna real na stack.
7. Colocar lógica de página dentro de Patterns ou Core.
8. Fazer alterações visuais que não atualizem stories, documentação e testes.
9. Usar apenas a cor para indicar estado, erro, criptografia ou confiança.
10. Exibir um indicador de “seguro” sem distinguir o que foi realmente verificado.
11. Ignorar estados de teclado, foco, loading, erro, vazio ou conteúdo longo.
12. Introduzir APIs incompatíveis sem plano de migração e política de versão.
13. Criar abstrações para um único uso sem motivo arquitetural claro.
14. Alterar tokens globais para resolver um problema local de layout.
15. Tratar testes automatizados de acessibilidade como prova suficiente de conformidade.

## 9. Maturidade, versionamento e deprecação

### 9.1 Status

- **Draft:** proposta ainda em elaboração.
- **Experimental:** implementação exploratória; API pode mudar.
- **Beta:** uso permitido com limitações conhecidas e feedback esperado.
- **Stable:** API documentada e coberta por testes; mudanças seguem política de compatibilidade.
- **Deprecated:** não recomendado para novos usos; existe alternativa ou plano de remoção.
- **Removed:** removido do sistema; a migração aplicável deve estar documentada.

### 9.2 Versionamento

Usar versionamento semântico quando o Design System for consumido como pacote versionado:
- **MAJOR:** quebra de compatibilidade de API, tokens públicos ou comportamento contratado.
- **MINOR:** nova capacidade compatível.
- **PATCH:** correção compatível.

Se o Design System permanecer no mesmo repositório e release do aplicativo, manter changelog e comunicar o impacto das mudanças mesmo sem publicar um pacote separado.

### 9.3 Política de deprecação

Uma deprecação deve informar motivo, alternativa, impacto e instruções de migração. Não remover uma API Stable silenciosamente. O prazo de remoção deve considerar o ciclo de release do aplicativo e o custo de migração. Exceções urgentes por segurança devem ser registradas e comunicadas explicitamente.

## 10. Governança e processo de revisão

O processo operacional detalhado e seus templates devem ficar em `CONTRIBUTING.md`. Esta seção define os princípios mínimos.

### 10.1 Gatilhos de revisão

Revisão do Design System é obrigatória para:
- Novo componente Core ou Product.
- Novo Pattern ou Layout reutilizável.
- Novo token ou alteração semântica de token existente.
- Nova variante ou novo tamanho público.
- Mudança de API ou comportamento.
- Breaking change ou deprecação.
- Mudança de acessibilidade, conteúdo ou tema com impacto transversal.
- Alteração no pipeline de tokens.

### 10.2 Papéis

| Papel | Responsabilidade |
|---|---|
| DS Lead / mantenedor | Responsável pela decisão final e coerência arquitetural |
| DS Engineer | Implementação, API, tokens, testes e manutenção |
| A11y Champion | Avaliação de acessibilidade; pode bloquear mudanças que introduzam regressões críticas |
| Product Designer / proponente | Problema, evidências de uso, proposta e estados visuais |
| Content Designer / responsável por localização | Microcopy, i18n, truncamento e RTL |
| Design Ops, quando aplicável | Biblioteca de design, tokens e paridade entre design e código |
| Produto/engenharia consumidora | Validação do uso real e impacto nos fluxos |

Em equipes pequenas, uma pessoa pode exercer mais de um papel, mas a responsabilidade por cada gate precisa estar explícita.

### 10.3 Seis gates

1. **Fit de camada:** o elemento está na camada correta e sua responsabilidade está clara?
2. **Reuso e duplicação:** existe componente semelhante? A regra dos três usos foi cumprida ou a exceção foi justificada?
3. **Tokens e theming:** usa tokens adequados e funciona nos temas e densidades suportados?
4. **API e acessibilidade:** estados, teclado, foco, semântica e leitores de tela foram avaliados?
5. **Conteúdo e localização:** microcopy, tradução, truncamento e RTL foram considerados?
6. **Documentação e testes:** exemplos, testes, changelog e notas de migração estão completos?

### 10.4 Cadência e SLA sugeridos

- Revisão assíncrona por pull request como caminho padrão.
- Reunião semanal de Design System, de até 45 minutos, para decisões que não se resolvam de forma assíncrona.
- Revisão mensal de arquitetura para tokens, breaking changes, deprecações e métricas.
- Meta inicial de feedback em até três dias úteis. Esse prazo é uma meta operacional, não uma garantia quando houver bloqueios ou dependências externas.

## 11. Definition of Done

Um componente ou Pattern só é considerado pronto quando os itens aplicáveis forem concluídos:

- [ ] Camada e status de maturidade definidos.
- [ ] Caso de uso real e decisão de reuso documentados.
- [ ] API, variantes, estados e limitações descritos.
- [ ] Tokens utilizados documentados; valores arbitrários justificados.
- [ ] Temas e densidades suportados verificados.
- [ ] Teclado, foco, semântica e acessibilidade testados.
- [ ] Strings preparadas para localização; RTL considerado.
- [ ] Exemplos ou stories cobrindo variantes e estados relevantes.
- [ ] Testes unitários e de acessibilidade adicionados conforme aplicável.
- [ ] Regressão visual coberta quando houver infraestrutura disponível.
- [ ] Documentação atualizada com exemplos e anti-patterns.
- [ ] Changelog e migração adicionados quando necessários.
- [ ] Gates de revisão aprovados.
- [ ] Paridade entre design e código validada, se houver biblioteca visual de design.

Nenhuma meta de ferramenta deve substituir a avaliação do comportamento real. Por exemplo, zero violações automatizadas não garante acessibilidade completa.

## 12. Métricas de saúde

As métricas devem ser coletadas de forma consistente e usadas para orientar melhorias, não para incentivar números artificiais. Metas iniciais são hipóteses a validar após estabelecer uma linha de base.

| Métrica | Como medir | Meta inicial sugerida |
|---|---|---|
| Cobertura de tokens | Propriedades visuais cobertas por tokens / propriedades auditadas | ≥ 95% nas áreas migradas |
| Adoção do Design System | Telas ou fluxos elegíveis que usam componentes aprovados | ≥ 80% após a fase de migração |
| Componentes duplicados ou órfãos | Componentes equivalentes sem justificativa ou sem consumidores | 0 duplicatas conhecidas sem plano |
| Regressões críticas de acessibilidade | Regressões críticas encontradas por release | 0 conhecidas no release |
| Lead time de revisão | Tempo entre PR pronto para revisão e decisão | Mediana ≤ 3 dias úteis |
| Drift design-código | Diferenças relevantes entre biblioteca visual e implementação | Corrigir diferenças críticas no ciclo atual |
| Deprecações pendentes | Itens Deprecated sem migração ou decisão atualizada | Revisão mensal |
| Cobertura de estados | Componentes com estados relevantes documentados e testados | 100% dos componentes Stable |

## 13. Decisões arquiteturais e documentos relacionados

Manter um log de decisões (ADR ou `DECISIONS.md`) para escolhas com impacto duradouro, como biblioteca de componentes, estratégia de tokens, suporte a temas, dependências de primitives e política de compatibilidade.

Separar os documentos por finalidade:
- `EpistulaMailDesignSystem-v3.md` ou `ARCHITECTURE.md`: arquitetura, camadas, princípios e critérios.
- `CONTRIBUTING.md`: fluxo de contribuição, templates de PR/RFC, gates, responsáveis e SLAs.
- `ROADMAP.md`: sequência de implementação e prioridades.
- `DECISIONS.md` ou diretório `adr/`: decisões arquiteturais e seus motivos.
- Documentação por componente: API, uso, estados, exemplos e testes.

### Template mínimo de RFC

Uma mudança relevante deve registrar:
1. Problema e contexto.
2. Evidências de uso ou necessidade.
3. Camada proposta e alternativas consideradas.
4. API e comportamento propostos.
5. Impacto em acessibilidade, temas, localização e segurança.
6. Plano de testes e documentação.
7. Compatibilidade, migração e versionamento.
8. Decisão, responsáveis e data.

## 14. Ordem recomendada de adoção

A implementação deve ser incremental. Não tentar migrar todo o catálogo de uma vez.

1. **Inventário:** identificar estilos, componentes, padrões e telas existentes; registrar duplicações e divergências.
2. **Foundations:** definir nomenclatura e tokens essenciais de cor, tipografia, espaçamento, sizing, foco, bordas e temas.
3. **Primitives e Utilities:** identificar lacunas reais e reutilizar bibliotecas maduras sempre que possível.
4. **Core essencial:** padronizar apenas componentes usados pelos fluxos existentes, começando por ações, campos, feedback, navegação e overlays necessários.
5. **Patterns recorrentes:** extrair composições comprovadas em fluxos reais.
6. **Product Components:** priorizar Account Rail, navegação de pastas, lista de mensagens, thread, reader, composer, destinatários e estados de segurança.
7. **Layouts & Pages:** migrar telas para as camadas estabilizadas.
8. **Governança contínua:** medir cobertura, regressões, adoção, duplicações e custo de manutenção; ajustar o roadmap.

A ordem exata pode mudar conforme o estado atual do código. Antes de criar um componente, verificar se já existe uma implementação, uma dependência apropriada ou um padrão que possa ser reaproveitado.

## 15. Critério de sucesso

O Design System será bem-sucedido quando tornar as interfaces do EpistulaMail mais consistentes e acessíveis, reduzir duplicação e custo de mudança, acelerar a implementação de fluxos de e-mail e comunicar com precisão estados importantes, especialmente os relacionados à privacidade e à segurança.

O sucesso não será medido pelo número de componentes catalogados. Será medido pela qualidade, adoção e confiabilidade dos componentes que resolvem problemas reais do produto.

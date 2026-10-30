# Roteiro do design system

A ordem de trabalho do design system do EpistulaMail, na sequência do v3 (§14). As decisões que sustentam cada item estão em `DECISIONS.md`; o fluxo para fazer uma mudança, em `CONTRIBUTING.md`. A linha de base das métricas está na página Governança e sai de `pnpm ds:check --metrics`.

## Feito em 28 out 2026 (US21)

1. **Inventário.** 43 páginas no registro, 310 tokens, 96 de 98 arquivos de componente do app ligados a uma página do design system.
2. **Fundamentos.**
   - Tokens em três níveis: 117 primitivos e 193 semânticos, sem mudar nenhum valor.
   - Domínios novos: foco, opacidade, ícones, breakpoints e densidade nomeada.
   - Temas claro, escuro e alto contraste.
   - Propriedades lógicas para RTL.
   - Página de variáveis em forma de catálogo.
   - Páginas de Arquitetura, Temas, Conteúdo e tradução, Layouts e Governança.
3. **Primitives e utilities.** Mapeados em `arquitetura.html`.
4. **Core.** Navegação lateral separada do que é do produto.
5. **Padrões.** Validação, confirmação, busca, seleção em lote, vazios e carregamento, com o modelo de padrão.
6. **Produto.** Contas e pastas separadas; estados de segurança distintos em `criptografia.html`.
7. **Fichas.** Cada página de componente tem a ficha: camada, status, onde é usado, fazer e não fazer, tokens, texto e tradução, testes, plataforma, relacionados e histórico.
8. **Governança.** `ds:check` no CI, `CONTRIBUTING.md`, `DECISIONS.md` e este roteiro.

## Próximo

Cada item tem uma evidência e pode virar uma proposta futura em <code>userstory/</code>.

| # | O quê | Evidência | Camada |
|---|---|---|---|
| 1 | Aplicar os estados de segurança no app: selos neutro, tracejado e pontilhado em `SecurityStatus.tsx` e em `securityBadges`. | Seis estados; o app tem só os selos de criptografia e de assinatura, sem os neutros, o tracejado e o pontilhado. | Produto |
| 2 | Extrair um helper de classes em `src/lib`. | 33 arquivos montam a lista na mão. | Utilities |
| 3 | Extrair um primitive para fechar com Esc e clique fora. | 9 arquivos tratam o Esc. | Primitives |
| 4 | Extrair um primitive de foco por setas. | 9 arquivos tratam as setas. | Primitives |
| 5 | Teste automático de acessibilidade e regressão visual das páginas do design system. | O `ds:check` confere tokens e registro, mas não pixels nem leitor de tela. | Governança |
| 6 | Decidir o destino dos 6 tokens sem uso: usar ou tirar. | `--color-border-warning`, `--space-0`, `--z-card`, `--z-dialog`, `--z-modal`, `--easing-exit`. | Fundamentos |
| 7 | Trocar os 25 valores fixos de `components.css` por tokens. | `pnpm ds:check --metrics --list` lista cada um. | Fundamentos |
| 8 | Preparar o app para tradução e validar o RTL com um idioma real. | O app só tem pt-BR; o espelhamento foi conferido só nos componentes. | Conteúdo |
| 9 | Conferir o alto contraste com um sistema de verdade e a seta do ícone no RTL. | Só foi testado por atributo. | Temas |
| 10 | Revisar os status de maturidade. | Os status Beta de `arquivos` e `criptografia` e os quatro padrões novos são estimativas. | Governança |

## Como priorizar

1. O que afeta a segurança percebida vem primeiro (item 1).
2. O que tem três ou mais usos medidos vem a seguir (itens 2 a 4).
3. O que dá segurança às próximas mudanças vem depois (itens 5 a 7).
4. O que depende de um idioma ou de um ambiente que ainda não existe espera (itens 8 e 9).

O sucesso não se mede pelo número de componentes catalogados, e sim pela qualidade, pela adoção e pela confiabilidade dos que resolvem problemas reais do produto.

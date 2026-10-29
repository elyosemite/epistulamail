# Decisões do design system

O registro das escolhas com efeito duradouro no design system do EpistulaMail: biblioteca, estratégia de tokens, temas, primitives e compatibilidade. O modelo de proposta está em `CONTRIBUTING.md`. Cada entrada diz o contexto, a decisão e o que muda por causa dela.

As decisões 0001 e 0002 vêm de antes do v3 e estão aqui para o registro ficar completo. As demais são de 28 out 2026, da entrega que aplicou o `EpistulaMailDesignSystem-v3.md` (`userstory/0021-design-system-v3.md`).

## 0001 · A base é o Atlassian Design System

- **Status:** em vigor, anterior ao v3.
- **Contexto:** o produto é uma caixa de entrada densa, e o ADS já resolve bem estrutura azul nítida, superfícies neutras e tipografia para telas cheias.
- **Decisão:** usar o ADS como referência, adaptado a uma caixa de entrada que se lê rápido e se responde com um gesto. Meta de WCAG 2.2 AA, temas claro e escuro.
- **Efeito:** os nomes de várias peças seguem o ADS ("Side Navigation", "Lozenge").

## 0002 · O sprite de ícones vive dentro de `docs.js` (L2)

- **Status:** em vigor, anterior ao v3.
- **Decisão:** não existe `icons.svg`. O app (`vite.config.ts`), o site (`site/build.mjs`) e as docs (`docs-site/scripts/sync-ds.mjs`) extraem o sprite de `docs.js` por expressão regular.
- **Efeito:** não mudar o formato de `insertAdjacentHTML('afterbegin', "...")` sem atualizar os três extratores.

## 0003 · `tokens.css` é a fonte de verdade, sem Figma nem Style Dictionary

- **Contexto:** o v3 recomenda um fluxo Figma → Style Dictionary → CSS, mas diz que, sem essas ferramentas, basta documentar a fonte efetiva e manter a separação entre os níveis.
- **Decisão:** o projeto não usa biblioteca de design. `design-system/tokens.css` é a fonte. A conferência é o `pnpm ds:check`, que valida nomes, referências, paridade do tema escuro, uso da paleta, breakpoints e tokens sem uso.
- **Efeito:** o `ds:check` roda no CI. Se um dia houver biblioteca de design, a decisão é revista.

## 0004 · Tokens em três níveis, com paleta primitiva `--palette-*`

- **Contexto:** os tokens misturavam valor bruto e significado: 178 tokens, quase todos com hexadecimal solto.
- **Decisão:** nível 1 são 117 primitivos `--palette-<família>-<tom>`, só de `tokens.css`; nível 2 são os semânticos, que apontam para a paleta e são o que se usa; nível 3 são ajustes de um componente, declarados no próprio `components.css`. Os dois temas usam a mesma paleta.
- **Efeito:** nenhum valor mudou: os 178 tokens originais resolvem para o mesmo valor nos dois temas, conferido no navegador. O `ds:check` falha se `components.css`, `docs.css`, `docs.js` ou o app usarem `--palette-*`. Os nomes dos primitivos são internos e podem mudar de tom quando entrar uma cor nova; os semânticos são públicos.

## 0005 · Camadas e maturidade por página

- **Decisão:** cada página do design system mora numa de cinco camadas (Fundamentos, Core, Padrões, Produto, Layouts e páginas) e declara um status (Rascunho, Experimental, Beta, Estável, Descontinuado, Removido) no registro `EPISTULA_PAGES`.
- **Efeito:** o status aparece no índice e ao lado do título. Páginas novas começam em Beta ou abaixo. Os status atuais de `arquivos` e `criptografia` (Beta) são uma estimativa e podem ser corrigidos no registro.

## 0006 · A navegação lateral se separa de pastas e de contas

- **Contexto:** a página "Sidebar" misturava a estrutura genérica, a árvore de pastas e a coluna de contas, o exemplo que o v3 (§3) usa para mostrar a separação.
- **Decisão:** `navegacao.html` (Core), `contas.html` e `pastas.html` (Produto). `sidebar.html` foi removida.
- **Efeito:** os comentários `// DS:` em `src/` apontam para as páginas novas.

## 0007 · Primitives: nenhuma biblioteca, e três lacunas reais

- **Contexto:** o v3 manda reutilizar uma biblioteca madura e não criar abstração sem lacuna demonstrada. O app só tem React.
- **Decisão:** não adotar biblioteca de componentes agora. Mapear o que existe (`arquitetura.html`) e extrair só o que passa dos três usos. Medido em 28 out 2026: nove arquivos tratam o Esc, nove tratam as setas e 33 montam a lista de classes na mão.
- **Efeito:** os três candidatos (fechar com Esc, setas, helper de classes) estão em `ROADMAP.md`. Refatorar o app é trabalho à parte.

## 0008 · Propriedades lógicas no CSS, com exceções marcadas

- **Decisão:** `components.css` usa `inline-start` e `inline-end` no lugar de esquerda e direita. As exceções (o "ok" do checkbox, o giro do spinner, o que se posiciona por coordenadas do ponteiro, a área segura da tela) têm o comentário "físico" na mesma linha.
- **Efeito:** o `ds:check` só deixa passar as marcadas. Em linguagem da esquerda para a direita nada mudou, conferido por estilos computados e posições de cada elemento. A validação com um idioma RTL real fica para quando o app for traduzido.

## 0009 · Alto contraste reaponta tokens, e não cria valores

- **Decisão:** `@media (prefers-contrast: more)` e `data-contrast="more"` reapontam bordas, texto de apoio, ícones sutis e o anel de foco para tokens mais fortes. Vale no claro e no escuro.
- **Efeito:** nenhum componente precisa saber do alto contraste. Não foi conferido com um sistema em alto contraste de verdade, só pelo atributo.

## 0010 · Estados de segurança distintos (Beta)

- **Decisão:** seis estados com texto, ícone e forma próprios (em trânsito, neste computador, ponta a ponta, assinatura válida, identidade não verificada, desconhecido). Verde só para assinatura verificada com chave aceita. O que não foi confirmado tem borda tracejada; o desconhecido, pontilhada.
- **Efeito:** são peças L85 a L87 em `components.css`, documentadas em `criptografia.html`. Ainda não estão no app: `src/ui/SecurityStatus.tsx` e `securityBadges` em `src/lib/crypto.ts` seguem como antes.

## 0011 · Foto de exemplo dos avatares

- **Decisão:** as páginas do design system usam as fotos em domínio público (CC0) de `design-system/images/people/`, as mesmas do site. Nada de serviço externo, porque o hash do endereço de e-mail identifica a pessoa.
- **Efeito:** o seletor de cor da conta continua sem foto, porque ali a cor é a informação.

## 0012 · Texto das páginas sem classes CSS

- **Decisão:** a explicação das páginas fala em linguagem de produto. Os nomes de classe ficam numa linha "Classes" no fim de cada seção.

## 0013 · Densidade: nenhuma além das atuais

- **Decisão:** a lista tem três densidades (Confortável, Padrão, Compacta) e os controles têm duas alturas. O v3 diz "se a pesquisa de uso justificar"; não há pesquisa que peça outra.
- **Efeito:** uma densidade nova entraria como valor novo dos tokens `--row-height-*` e `--control-height*`, e não como componente.

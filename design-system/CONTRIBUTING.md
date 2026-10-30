# Como contribuir com o design system

Este arquivo é o fluxo operacional do design system do EpistulaMail. A arquitetura, as camadas e os critérios estão em `EpistulaMailDesignSystem-v3.md` e na página `governanca.html`. As decisões com impacto duradouro ficam em `DECISIONS.md`, e a ordem de trabalho em `ROADMAP.md`.

## Quando uma mudança passa por revisão

Passa por revisão (sempre) qualquer um destes casos:

- novo componente Core ou Produto;
- novo padrão ou novo layout reutilizável;
- novo token, ou mudança de significado de um token existente;
- nova variante ou novo tamanho público;
- mudança de API ou de comportamento;
- mudança que quebra o que já existe, ou descontinuação;
- mudança de acessibilidade, de texto ou de tema com efeito em várias peças;
- mudança no jeito de gerar ou conferir os tokens.

Correção de erro de digitação e ajuste de texto de uma página só não precisam de proposta.

## Fluxo

1. **Rascunho.** Crie `design-system/rascunho-<tema>.html` só com tokens e componentes que já existem, mostrando todos os estados nos dois temas. O CSS novo vai em `components.css`, só com tokens.
2. **Aprovação.** Mostre o rascunho a quem mantém o design system. Nada vai para `src/`, `site/` ou `docs-site/` antes disso.
3. **Incorporação.** Mova as peças para a página do componente (ou crie `<id>.html` e registre em `EPISTULA_PAGES`, em `docs.js`, com camada e status). Apague o rascunho e registre o que foi decidido em <code>DECISIONS.md</code>.
4. **Implementação.** Use as mesmas classes no app, no site e nas docs.
5. **Conferência.** Rode `pnpm ds:check` e deixe tudo sem erro.

## Papéis

O projeto tem uma pessoa mantendo o design system, então os papéis do v3 se acumulam. A responsabilidade por cada gate continua explícita:

| Papel | Quem faz hoje | O que decide |
|---|---|---|
| Responsável pelo design system | quem mantém o repositório | decisão final e coerência da arquitetura |
| Implementação | quem abre a mudança | API, tokens, testes e manutenção |
| Acessibilidade | quem abre a mudança, com revisão de quem mantém | pode bloquear mudança que piore um fluxo crítico |
| Texto e tradução | quem abre a mudança | microtexto, truncamento, tradução e RTL |

## Os seis gates

Antes de aprovar, responda sim às seis perguntas (ou registre a exceção):

1. **Camada.** O elemento está na camada certa, com uma responsabilidade clara?
2. **Reuso.** Já existe algo parecido? Há três usos reais, ou a exceção está justificada?
3. **Tokens e temas.** Usa só tokens semânticos e funciona no claro, no escuro e em alto contraste?
4. **API e acessibilidade.** Teclado, foco, semântica, leitor de tela e estados foram avaliados?
5. **Texto e tradução.** O texto pode ser traduzido? Truncamento, plural e RTL foram considerados?
6. **Documentação e testes.** Há exemplos, a ficha do componente, a conferência e o registro em <code>DECISIONS.md</code>/<code>CHANGELOG.md</code>?

## Definição de pronto

- [ ] Camada e status definidos.
- [ ] Caso de uso real e decisão de reuso registrados.
- [ ] API, variantes, estados e limites descritos.
- [ ] Tokens documentados; valor fixo justificado.
- [ ] Claro, escuro e alto contraste verificados.
- [ ] Teclado, foco e semântica testados na mão.
- [ ] Texto preparado para tradução; RTL considerado.
- [ ] Exemplos cobrindo as variantes e os estados.
- [ ] Ficha do componente preenchida (fazer e não fazer, texto, testes, relacionados, histórico).
- [ ] `pnpm ds:check` sem erro.
- [ ] Changelog e migração, se algo mudou para quem já usa.
- [ ] Gates aprovados.

Teste automático ajuda, mas não substitui conferir com teclado e leitor de tela.

## Maturidade e descontinuação

Cada página declara o status: Rascunho, Experimental, Beta, Estável, Descontinuado ou Removido.

Uma descontinuação diz o motivo, a alternativa, o impacto e como migrar. Nada Estável some sem aviso. O prazo considera o ciclo de versões do app. Exceção por segurança é registrada em `DECISIONS.md` e avisada na hora.

## Modelo de proposta (RFC)

Uma mudança relevante registra:

1. Problema e contexto.
2. Evidência de uso ou necessidade.
3. Camada proposta e alternativas consideradas.
4. API e comportamento propostos.
5. Efeito em acessibilidade, temas, tradução e segurança.
6. Plano de testes e de documentação.
7. Compatibilidade, migração e versão.
8. Decisão, responsável e data.

Aprovada, a proposta vira uma entrada em `DECISIONS.md`.

## Ritmo

- Revisão por pull request é o caminho comum.
- A meta é dar retorno em até três dias úteis. É uma meta, não uma garantia.
- Uma vez por mês, olhe a lista de descontinuações pendentes e as métricas de `pnpm ds:check --metrics`.

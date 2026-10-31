# Site do Epistula

A landing page pública do Epistula. Página estática, sem framework e sem dependências próprias: `build.mjs` monta `dist/` com a página, o CSS do site, o `tokens.css` e o `components.css` do [design system](../design-system/) (copiados, nunca editados aqui), as fontes empacotadas e o sprite de ícones do DS.

A página não carrega nada de terceiros (nem fontes), não tem cookies nem rastreamento. Os cabeçalhos de segurança, inclusive a CSP, estão em `vercel.json`.

## Rodar localmente

Na raiz do repositório, `pnpm install` (as fontes vêm dos pacotes `@fontsource`). Depois:

```powershell
cd site
node build.mjs --serve   # http://localhost:4321
```

## Estrutura

- `index.html`: a página. As "fotos" do app são o próprio markup do DS (as mesmas classes do app), dentro de `.site-panel`; a captura da caixa de entrada vem de `images/epistula-janela.webp`.
- `site.css`: só o que é do site (cabeçalho, hero, faixas, linhas de recurso alternadas, cartões, rodapé), sempre com tokens do DS. Prefixo `site-`.
- `site.js`: o download com o menu do sistema e a troca de tema. `theme.js`: aplica o tema salvo antes de desenhar.

## Publicação (Vercel)

O workflow `.github/workflows/site.yml` roda só quando `site/`, `design-system/tokens.css`, `components.css` ou `docs.js` mudam. Push no `main` publica em produção; pull request publica uma prévia (o endereço aparece no resumo da execução).

A Vercel não é ligada ao GitHub: quem publica é o workflow, com a CLI da Vercel e um token. A Vercel nunca lê o repositório.

Configuração (já feita; refazer só ao trocar de conta ou de token):

1. Na Vercel, **Account Settings → Tokens**: um token com escopo na conta.
2. O projeto `epistula` foi criado pela API, sem repositório ligado e com o preset "Other":
   `POST https://api.vercel.com/v11/projects?teamId=<time>` com `{"name":"epistula","framework":null}`. O `teamId` é o `defaultTeamId` de `GET https://api.vercel.com/v2/user`.
3. No GitHub, em **Settings → Secrets and variables → Actions**: `VERCEL_TOKEN`, `VERCEL_ORG_ID` (o id do time) e `VERCEL_PROJECT_ID` (o `id` devolvido na criação).

Ao vencer o token, gere outro e troque só o segredo `VERCEL_TOKEN`. Sem os segredos, o workflow monta o site e para antes de publicar.

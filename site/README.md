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

- `index.html`: a página. As "fotos" do app são o próprio markup do DS (as mesmas classes do app), dentro de `.site-shot`.
- `site.css`: só o que é do site (cabeçalho, hero, faixas, linhas de recurso alternadas, cartões, rodapé), sempre com tokens do DS. Prefixo `site-`.
- `site.js`: a entrada das linhas ao rolar e a troca de tema. `theme.js`: aplica o tema salvo antes de desenhar.

## Publicação (Vercel)

O workflow `.github/workflows/site.yml` roda só quando `site/`, `design-system/tokens.css`, `components.css` ou `docs.js` mudam. Push no `main` publica em produção; pull request publica uma prévia (o endereço aparece no resumo da execução).

Para ligar:

1. Na Vercel, crie um projeto (pode ser vazio, sem conectar o repositório: quem publica é o workflow). Em **Settings → General**, deixe **Root Directory** em branco e o **Framework Preset** como "Other".
2. Gere um token em **Account Settings → Tokens**.
3. Pegue o `orgId` e o `projectId`: rode `npx vercel@63.1.2 link` dentro de `site/` e copie de `site/.vercel/project.json` (a pasta `.vercel` fica fora do git).
4. No GitHub, em **Settings → Secrets and variables → Actions**, crie `VERCEL_TOKEN`, `VERCEL_ORG_ID` e `VERCEL_PROJECT_ID`.

Sem os segredos, o workflow monta o site e para antes de publicar.

// Documentação do EpistulaMail (docs.epistulamail.com): Starlight com o design system do app.
// Estrutura como a do OpenClaw: "Comece aqui" (páginas de passos) e "Documentação" (seções; ao entrar numa, a sidebar
// mostra só as sub-páginas dela, ver src/components/Sidebar.astro).
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

const REPO = "https://github.com/elyosemite/epistulamail";
// Perfis nas redes: os botões só aparecem quando o endereço existir.
const X_URL = "https://x.com/theyurimelo";
const DISCORD_URL = "https://discord.gg/5NsPCnAUE";

/** Rótulo em português (idioma padrão) e nos outros idiomas. */
const t = (pt, en, es, fr, it) => ({ label: pt, translations: { en, es, fr, it } });
/** Uma seção da Documentação: as páginas da pasta, recolhida até a pessoa entrar nela. */
const section = (dir, ...labels) => ({ ...t(...labels), collapsed: true, items: [{ autogenerate: { directory: `guia/${dir}` } }] });

export default defineConfig({
  site: "https://docs.epistulamail.com",
  integrations: [
    starlight({
      title: "EpistulaMail",
      description: "Documentação do EpistulaMail: instalar, usar e contribuir.",
      logo: { src: "./src/assets/epistulamail-logo.png", alt: "" },
      favicon: "/favicon.png",
      defaultLocale: "root",
      locales: {
        root: { label: "Português (Brasil)", lang: "pt-BR" },
        en: { label: "English", lang: "en" },
        es: { label: "Español", lang: "es" },
        fr: { label: "Français", lang: "fr" },
        it: { label: "Italiano", lang: "it" },
      },
      social: [
        { icon: "github", label: "GitHub", href: REPO },
        ...(X_URL ? [{ icon: "x.com", label: "X", href: X_URL }] : []),
        ...(DISCORD_URL ? [{ icon: "discord", label: "Discord", href: DISCORD_URL }] : []),
      ],
      editLink: { baseUrl: `${REPO}/edit/main/docs-site/` },
      customCss: [
        "@fontsource/inter/400.css",
        "@fontsource/inter/500.css",
        "@fontsource/inter/600.css",
        "@fontsource/inter/700.css",
        "@fontsource/jetbrains-mono/400.css",
        "./src/styles/ds-tokens.css",
        "./src/styles/ds-pieces.css",
        "./src/styles/theme.css",
        "./src/styles/proposta-escuro.css",
      ],
      components: {
        Sidebar: "./src/components/Sidebar.astro",
        SocialIcons: "./src/components/SocialIcons.astro",
        ThemeSelect: "./src/components/ThemeSelect.astro",
        LanguageSelect: "./src/components/LanguageSelect.astro",
        Pagination: "./src/components/Pagination.astro",
      },
      sidebar: [
        {
          ...t("Comece aqui", "Get Started", "Primeros pasos", "Bien démarrer", "Per iniziare"),
          items: [
            { slug: "index", ...t("Visão geral", "Overview", "Descripción general", "Vue d'ensemble", "Panoramica") },
            { slug: "por-que-o-epistulamail", ...t("Por que o EpistulaMail", "Why EpistulaMail", "Por qué EpistulaMail", "Pourquoi EpistulaMail", "Perché EpistulaMail") },
            { slug: "inicio-rapido", ...t("Início rápido", "Quickstart", "Inicio rápido", "Démarrage rapide", "Avvio rapido") },
            { slug: "instalacao", ...t("Instalação", "Installation", "Instalación", "Installation", "Installazione") },
            { slug: "primeira-configuracao", ...t("Primeira configuração", "Onboarding", "Primera configuración", "Première configuration", "Prima configurazione") },
            { slug: "configuracao", ...t("Configuração", "Configuration", "Configuración", "Configuration", "Configurazione") },
          ],
        },
        {
          ...t("Documentação", "Documentation", "Documentación", "Documentation", "Documentazione"),
          items: [
            { slug: "guia/conceitos", ...t("Conceitos", "Concepts", "Conceptos", "Concepts", "Concetti") },
            { slug: "guia/contas" },
            { slug: "guia/ler-e-escrever" },
            { slug: "guia/busca" },
            { slug: "guia/regras" },
            {
              ...t("Criptografia", "Encryption", "Cifrado", "Chiffrement", "Crittografia"),
              collapsed: true,
              items: [
                { slug: "guia/criptografia/como-funciona" },
                { slug: "guia/criptografia/openpgp" },
                { slug: "guia/criptografia/s-mime" },
              ],
            },
            { slug: "guia/arquivos/abrir-um-arquivo", ...t("Arquivos de e-mail", "Email archives", "Archivos de correo", "Archives d'e-mails", "Archivi di posta") },
            { slug: "guia/privacidade" },
            section("contribuir", "Contribuir", "Contributing", "Contribuir", "Contribuer", "Contribuire"),
            { slug: "guia/versoes/historico", ...t("Versões", "Releases", "Versiones", "Versions", "Versioni") },
            { slug: "guia/ajuda" },
          ],
        },
      ],
    }),
  ],
});

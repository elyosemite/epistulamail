import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
// Fontes do DS empacotadas com o app: nenhuma requisição ao Google, e a tela abre mesmo sem rede.
// tokens.css e components.css entram pelo index.html, antes do JavaScript.
import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-700.css";
import "@fontsource/inter/latin-ext-400.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import { App } from "./App";
import { applyAppearance, cachedAppearance } from "./lib/appearance";
import { IconSprite } from "./ui/Icon";
import { TooltipLayer } from "./ui/TooltipLayer";

// Tema e fundo da última abertura antes do primeiro desenho; o App confirma com o banco em seguida.
applyAppearance(cachedAppearance());

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <IconSprite />
    <App />
    <TooltipLayer />
  </StrictMode>,
);

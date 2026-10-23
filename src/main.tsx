import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../design-system/tokens.css";
import "../design-system/components.css";
import { App } from "./App";
import { IconSprite } from "./ui/Icon";
import { TooltipLayer } from "./ui/TooltipLayer";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <IconSprite />
    <App />
    <TooltipLayer />
  </StrictMode>,
);

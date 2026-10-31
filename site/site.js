// Landing page: o botão de download escolhe o pacote do sistema e o botão do cabeçalho troca o tema.
(() => {
  // Download (L81): a ação principal baixa o pacote do sistema; o chevron abre o menu com as opções.
  const downloads = document.querySelectorAll("[data-download]");
  const isLinux = () =>
    !/Android|iPhone|iPad|iPod/i.test(navigator.userAgent) &&
    /Linux/i.test(navigator.userAgentData?.platform || navigator.platform || navigator.userAgent);
  for (const download of downloads) {
    const main = download.querySelector("[data-download-main]");
    const toggle = download.querySelector("[data-download-toggle]");
    const menu = download.querySelector("[data-download-menu]");
    const linux = download.querySelector("[data-download-linux-appimage]");
    if (main && linux?.getAttribute("href") && isLinux()) main.setAttribute("href", linux.getAttribute("href"));
    for (const link of menu.querySelectorAll('[role="menuitem"]')) if (!link.getAttribute("href")) link.hidden = true;
    const links = () => [...menu.querySelectorAll('[role="menuitem"]')].filter((link) => !link.hidden && link.getAttribute("href"));
    const close = (focus) => {
      menu.hidden = true;
      toggle.setAttribute("aria-expanded", "false");
      if (focus) toggle.focus();
    };
    const open = () => {
      menu.hidden = false;
      toggle.setAttribute("aria-expanded", "true");
      links()[0]?.focus();
    };
    toggle.addEventListener("click", () => (menu.hidden ? open() : close(false)));
    toggle.addEventListener("keydown", (e) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        open();
      } else if (e.key === "Escape") {
        close(false);
      }
    });
    menu.addEventListener("keydown", (e) => {
      const all = links();
      const index = all.indexOf(document.activeElement);
      if (e.key === "ArrowDown") {
        e.preventDefault();
        all[(index + 1) % all.length]?.focus();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        all[(index - 1 + all.length) % all.length]?.focus();
      } else if (e.key === "Home") {
        e.preventDefault();
        all[0]?.focus();
      } else if (e.key === "End") {
        e.preventDefault();
        all.at(-1)?.focus();
      } else if (e.key === "Escape") {
        e.preventDefault();
        close(true);
      } else if (e.key === "Tab") {
        close(false);
      }
    });
    document.addEventListener("click", (e) => {
      if (!download.contains(e.target)) close(false);
    });
  }

  const root = document.documentElement;

  // Tema: segue o sistema até a pessoa escolher; a escolha fica só neste navegador.
  const button = document.getElementById("site-theme");
  if (!button) return;
  const isDark = () => (root.dataset.theme ? root.dataset.theme === "dark" : matchMedia("(prefers-color-scheme: dark)").matches);
  const sync = () => {
    const dark = isDark();
    button.setAttribute("aria-label", dark ? "Usar o tema claro" : "Usar o tema escuro");
    button.dataset.tip = button.getAttribute("aria-label");
    button.querySelector("use")?.setAttribute("href", dark ? "#i-sun" : "#i-moon");
  };
  button.addEventListener("click", () => {
    root.dataset.theme = isDark() ? "light" : "dark";
    try { localStorage.setItem("epistula-site-theme", root.dataset.theme); } catch { /* sem armazenamento: vale só nesta visita */ }
    sync();
  });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", sync);
  sync();
})();

// Blog: filtro por categoria no índice (sem recarregar a página).
(() => {
  const buttons = document.querySelectorAll(".site-blog__filter");
  if (!buttons.length) return;
  const items = document.querySelectorAll(".site-blog [data-tag]");
  for (const button of buttons) {
    button.addEventListener("click", () => {
      const tag = button.dataset.filter;
      for (const other of buttons) other.setAttribute("aria-pressed", String(other === button));
      for (const item of items) item.hidden = tag !== "Todos" && item.dataset.tag !== tag;
    });
  }
})();

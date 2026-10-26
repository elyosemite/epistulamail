// Landing page: as linhas de recurso aparecem ao chegar à tela, e o botão do cabeçalho troca o tema.
(() => {
  const root = document.documentElement;

  // Entrada ao rolar. Sem IntersectionObserver (ou com menos movimento pedido), tudo já aparece.
  const items = document.querySelectorAll(".site-reveal");
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (still || !("IntersectionObserver" in window)) {
    items.forEach((el) => el.classList.add("is-visible"));
  } else {
    const seen = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        entry.target.classList.add("is-visible");
        seen.unobserve(entry.target);
      }
    }, { rootMargin: "0px 0px -12% 0px" });
    items.forEach((el) => seen.observe(el));
  }

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

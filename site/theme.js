// Marca que o JavaScript roda (as entradas ao rolar só escondem as seções com ele).
document.documentElement.classList.add("js");
// Antes de desenhar: o tema escolhido nesta máquina (sem ele, o do sistema). Fora do site.js para não piscar.
try {
  const saved = localStorage.getItem("epistula-site-theme");
  if (saved === "light" || saved === "dark") document.documentElement.dataset.theme = saved;
} catch { /* sem armazenamento: segue o sistema */ }

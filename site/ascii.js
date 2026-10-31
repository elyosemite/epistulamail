// O planeta em caracteres do herói. Não é uma biblioteca: é um campo de caracteres
// desenhado no <pre> e atualizado a cada quadro, com uma esfera, um anel inclinado,
// uma onda na superfície e um ripple que segue o ponteiro. O anel passa por trás da
// esfera na metade de cima e por cima dela na metade da frente. Pausa fora da tela,
// com a aba escondida e com movimento reduzido; nesse caso, desenha um quadro parado.
(() => {
  const host = document.querySelector("[data-ascii]");
  if (!host) return;
  const hero = host.closest(".site-hero") || host;
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const root = document.documentElement;
  const media = matchMedia("(prefers-color-scheme: dark)");
  const isDark = () => (root.dataset.theme ? root.dataset.theme === "dark" : media.matches);
  let dark = isDark();

  const COLS = 240;
  const ROWS = 52;
  const CELL_W = 0.62;
  const PLANET_R = 24;
  const RING_A = 46;
  const RING_B = 11;
  const TILT = (-16 * Math.PI) / 180;
  const COS = Math.cos(TILT);
  const SIN = Math.sin(TILT);
  const DITHER = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [15, 7, 13, 5],
  ];
  const ASTEROIDS = [
    { x: 0.08, y: 0.2, vx: 4.2, vy: 1.2, r: 2.1 },
    { x: 0.86, y: 0.32, vx: -3.6, vy: 1.5, r: 2.3 },
    { x: 0.3, y: 0.08, vx: 2.4, vy: 0.8, r: 1.6 },
    { x: 0.96, y: 0.58, vx: -5.2, vy: -0.9, r: 1.8 },
    { x: 0.62, y: 0.12, vx: -2.8, vy: 1.1, r: 1.5 },
  ];

  const cells = [];
  for (let y = 0; y < ROWS; y++) {
    for (let x = 0; x < COLS; x++) {
      const h = Math.sin(x * 12.9898 + y * 78.233) * 43758.5453;
      cells.push({
        x,
        y,
        px: (x - COLS / 2) * CELL_W,
        py: y - ROWS / 2,
        star: h - Math.floor(h),
        threshold: (DITHER[y % 4][x % 4] + 0.5) / 16,
      });
    }
  }

  function render(time, pointer) {
    const chars = dark ? " .:-=+xX#8@" : " .·:-=+*#%@";
    let out = "";
    for (let i = 0; i < cells.length; i++) {
      const cell = cells[i];
      let sx = cell.px;
      let sy = cell.py;
      let highlight = 0;
      if (pointer) {
        const dx = cell.x - pointer.column;
        const dy = cell.y - pointer.row;
        const dist = Math.hypot(dx, dy);
        const o = dist / 7;
        if (dist > 0 && o < 2.2) {
          const shift = 7 * pointer.presence * o * Math.exp(0.5 - o * o) * Math.SQRT2;
          sx -= (dx / dist) * shift * CELL_W;
          sy -= (dy / dist) * shift;
          highlight = Math.exp(-(o * o)) * pointer.presence * 0.85;
        }
      }

      const r = Math.hypot(sx, sy);
      const inside = r < PLANET_R;
      let planet = 0;
      if (inside) {
        const surface =
          0.34 +
          0.24 * Math.sin(sy * 0.9 + Math.sin(sx * 0.18) * 1.4 + time * 0.15) +
          0.12 * Math.sin(sx * 0.7 - sy * 0.3 + time * 0.08);
        const rim = 0.38 * Math.exp(-(((r - PLANET_R + 1.5) / 1.5) ** 2));
        const light = Math.min(1.15, Math.max(0.45, 0.78 + (-sx * 0.4 - sy * 0.55) / PLANET_R));
        planet = Math.min(1, (surface + rim) * light);
      }

      const rx = sx * COS + sy * SIN;
      const ry = -sx * SIN + sy * COS;
      const e = Math.hypot(rx / RING_A, ry / RING_B);
      let ring =
        Math.exp(-(((e - 1) / 0.035) ** 2)) +
        0.6 * Math.exp(-(((e - 0.84) / 0.024) ** 2)) +
        0.4 * Math.exp(-(((e - 1.15) / 0.024) ** 2));
      if (ring > 0.02) {
        const angle = Math.atan2(ry, rx);
        ring *= 0.72 + 0.28 * Math.sin(angle * 7 + time * 0.28);
        if (inside && ry < 0) ring = 0;
      } else {
        ring = 0;
      }

      let asteroid = 0;
      for (const a of ASTEROIDS) {
        const ax = (((a.x * COLS + time * a.vx) % (COLS + 20)) + (COLS + 20)) % (COLS + 20) - 10;
        const ay = (((a.y * ROWS + time * a.vy) % (ROWS + 12)) + (ROWS + 12)) % (ROWS + 12) - 6;
        const dx = cell.x - ax;
        const dy = cell.y - ay;
        const dist = Math.hypot(dx, dy);
        if (dist < a.r) asteroid = Math.max(asteroid, 0.72 + 0.28 * (1 - dist / a.r));
        const vlen = Math.hypot(a.vx, a.vy) || 1;
        const proj = (dx * a.vx + dy * a.vy) / vlen;
        const perp = Math.abs((dx * a.vy - dy * a.vx) / vlen);
        if (proj < 0 && proj > -10 && perp < 0.9) {
          asteroid = Math.max(asteroid, 0.48 * (1 + proj / 10));
        }
      }
      const star = cell.star < 0.022
        ? (0.32 + 0.38 * (0.5 + 0.5 * Math.sin(time * 1.3 + cell.star * 90))) * (dark ? 1 : 0.7)
        : 0;
      let value = planet > 0 || ring > 0.02
        ? Math.max(planet, ring)
        : Math.max(asteroid, star);
      if (highlight) value = Math.min(1, value * (1 + highlight) + highlight * 0.15);
      if (value < 0.04) value = 0;
      value = Math.min(Math.max(value, 0), 1);
      const scaled = value * (chars.length - 1);
      const index = Math.min(chars.length - 1, Math.floor(scaled) + (scaled % 1 > cell.threshold ? 1 : 0));
      out += chars[index];
      if ((i + 1) % COLS === 0) out += "\n";
    }
    return out;
  }

  if (still) {
    const draw = () => { host.textContent = render(0, null); };
    const sync = () => { dark = isDark(); draw(); };
    draw();
    new MutationObserver(sync).observe(root, { attributes: true, attributeFilter: ["data-theme"] });
    media.addEventListener("change", sync);
    return;
  }

  let target = null;
  let pointer = null;
  let presence = 0;
  let raf = 0;
  let last = 0;
  let visible = true;

  const syncTheme = () => { dark = isDark(); };
  new MutationObserver(syncTheme).observe(root, { attributes: true, attributeFilter: ["data-theme"] });
  media.addEventListener("change", syncTheme);

  const frame = (time) => {
    if (!visible) return;
    const interval = target || presence > 0 ? 1000 / 30 : 100;
    if (time - last >= interval) {
      last = time;
      const rect = host.getBoundingClientRect();
      if (target && rect.width > 0 && rect.height > 0) {
        const x = target.x;
        const y = target.y;
        if (!pointer) pointer = { x, y, column: (x / rect.width) * COLS, row: (y / rect.height) * ROWS };
        pointer.x += (x - pointer.x) * 0.28;
        pointer.y += (y - pointer.y) * 0.28;
        pointer.column = (pointer.x / rect.width) * COLS;
        pointer.row = (pointer.y / rect.height) * ROWS;
      }
      presence += ((target ? 1 : 0) - presence) * 0.18;
      if (!target && presence < 0.01) presence = 0;
      const active = presence > 0 && pointer && rect.width > 0 && rect.height > 0
        ? { column: pointer.column, row: pointer.row, presence }
        : null;
      host.textContent = render(time / 1000, active);
    }
    raf = requestAnimationFrame(frame);
  };

  const stop = () => {
    if (!raf) return;
    cancelAnimationFrame(raf);
    raf = 0;
  };

  const start = () => {
    if (raf || !visible) return;
    last = 0;
    raf = requestAnimationFrame(frame);
  };

  const track = (event) => {
    if (event.pointerType === "touch") return;
    const rect = host.getBoundingClientRect();
    target = { x: event.clientX - rect.left, y: event.clientY - rect.top };
  };
  hero.addEventListener("pointermove", track);
  hero.addEventListener("pointerenter", track);
  hero.addEventListener("pointerleave", (event) => {
    if (event.pointerType !== "touch") target = null;
  });

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      visible = false;
      stop();
    } else {
      visible = true;
      start();
    }
  });

  const observer = new IntersectionObserver(([entry]) => {
    visible = entry?.isIntersecting ?? false;
    if (visible) start();
    else stop();
  }, { rootMargin: "160px 0px" });
  observer.observe(hero);

  host.textContent = render(0, null);
  start();
})();

// DS: menus.html · .tooltip.tooltip--floating ligado a [data-tip], mesmo comportamento do docs.js
import { useEffect, useRef } from "react";

const TIP_ID = "floating-tip";

export function TooltipLayer() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const tip = ref.current;
    if (!tip) return;
    let target: HTMLElement | null = null;

    const show = (el: HTMLElement) => {
      target = el;
      tip.textContent = el.dataset.tip ?? "";
      tip.hidden = false;
      const r = el.getBoundingClientRect();
      const t = tip.getBoundingClientRect();
      const pad = 8;
      const gap = 6;
      const x = Math.max(pad, Math.min(r.left + r.width / 2 - t.width / 2, innerWidth - pad - t.width));
      let y = r.bottom + gap;
      if (y + t.height > innerHeight - pad) y = r.top - gap - t.height;
      tip.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
      el.setAttribute("aria-describedby", TIP_ID);
    };
    const hide = () => {
      target?.removeAttribute("aria-describedby");
      target = null;
      tip.hidden = true;
    };
    const closestTip = (node: EventTarget | null) => (node instanceof Element ? node.closest<HTMLElement>("[data-tip]") : null);

    const onOver = (e: PointerEvent) => {
      const el = closestTip(e.target);
      if (el && el !== target) show(el);
      else if (!el && target) hide();
    };
    const onOut = (e: PointerEvent) => { if (!e.relatedTarget) hide(); };
    const onFocusIn = (e: FocusEvent) => {
      const el = closestTip(e.target);
      if (el?.matches(":focus-visible")) show(el);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") hide(); };

    document.addEventListener("pointerover", onOver);
    document.addEventListener("pointerout", onOut);
    document.addEventListener("focusin", onFocusIn);
    document.addEventListener("focusout", hide);
    document.addEventListener("click", hide);
    document.addEventListener("keydown", onKey);
    addEventListener("scroll", hide, true);
    return () => {
      document.removeEventListener("pointerover", onOver);
      document.removeEventListener("pointerout", onOut);
      document.removeEventListener("focusin", onFocusIn);
      document.removeEventListener("focusout", hide);
      document.removeEventListener("click", hide);
      document.removeEventListener("keydown", onKey);
      removeEventListener("scroll", hide, true);
    };
  }, []);

  return <div ref={ref} id={TIP_ID} className="tooltip tooltip--floating" role="tooltip" hidden />;
}

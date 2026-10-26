// DS: leitura.html "Corpo em HTML" · .message__frame (fundo --color-surface-paper)
//
// Segurança (US2, CA4): o HTML já chega sanitizado do Rust e com CSP própria. O iframe usa
// sandbox SEM allow-scripts, então nada do e-mail executa. allow-same-origin existe só para este
// componente medir a altura do conteúdo e interceptar cliques em links; sem scripts no iframe,
// a mesma origem não dá poder nenhum ao conteúdo. Links nunca navegam o iframe nem o app:
// vão para o navegador do sistema via backend, que só aceita http, https e mailto.
import { useCallback, useEffect, useRef } from "react";

interface MessageFrameProps {
  html: string;
  title: string;
  /** `text`: o texto do link, para o backend conferir se ele mostra outro domínio (US10, CA8). */
  onOpenLink: (url: string, text: string) => void;
}

export function MessageFrame({ html, title, onOpenLink }: MessageFrameProps) {
  const frame = useRef<HTMLIFrameElement>(null);
  const observer = useRef<ResizeObserver | null>(null);

  // O quadro cresce até a altura do conteúdo e o documento do e-mail não rola na vertical
  // (overflow-y: hidden no CSS do Rust): o único scroll vertical é o do painel de leitura.
  // A diferença entre innerHeight e clientHeight é a barra horizontal de e-mails largos.
  const resize = useCallback(() => {
    const win = frame.current?.contentWindow;
    const root = frame.current?.contentDocument?.documentElement;
    if (frame.current && win && root) {
      frame.current.style.height = `${root.scrollHeight + (win.innerHeight - root.clientHeight)}px`;
    }
  }, []);

  const onLoad = useCallback(() => {
    const doc = frame.current?.contentDocument;
    if (!doc) return;
    resize();
    observer.current?.disconnect();
    observer.current = new ResizeObserver(resize);
    observer.current.observe(doc.documentElement);
    if (doc.body) observer.current.observe(doc.body);
    doc.addEventListener("click", (event) => {
      const target = event.target instanceof Element ? event.target.closest("a[href]") : null;
      event.preventDefault();
      if (target) onOpenLink(target.getAttribute("href") ?? "", target.textContent ?? "");
    });
    doc.addEventListener("submit", (event) => event.preventDefault());
  }, [onOpenLink, resize]);

  useEffect(() => () => observer.current?.disconnect(), []);

  return (
    <iframe
      ref={frame}
      className="message__frame"
      title={title}
      sandbox="allow-same-origin"
      referrerPolicy="no-referrer"
      srcDoc={html}
      onLoad={onLoad}
    />
  );
}

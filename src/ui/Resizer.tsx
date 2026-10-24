// DS: espacamento.html "Com comandos e divisor" · .app-shell__resizer (role="separator", .is-dragging)
import { useRef, useState } from "react";
import { clampWidth, keyWidth, LIST_DEFAULT, widthPercent } from "../lib/listWidth";

interface ResizerProps {
  label: string;
  width: number;
  /** A cada movimento (atualiza a casca). */
  onChange: (width: number) => void;
  /** Ao soltar, ao usar o teclado ou no duplo clique (grava a preferência). */
  onCommit: (width: number) => void;
}

export function Resizer({ label, width, onChange, onCommit }: ResizerProps) {
  const [dragging, setDragging] = useState(false);
  const start = useRef<{ x: number; width: number } | null>(null);
  const latest = useRef(width);
  latest.current = width;

  return (
    <div
      className={["app-shell__resizer", dragging && "is-dragging"].filter(Boolean).join(" ")}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={widthPercent(width)}
      aria-valuetext={`${widthPercent(width)}%`}
      tabIndex={0}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        start.current = { x: e.clientX, width: latest.current };
        setDragging(true);
      }}
      onPointerMove={(e) => {
        if (!start.current) return;
        onChange(clampWidth(start.current.width + e.clientX - start.current.x));
      }}
      onPointerUp={() => {
        if (!start.current) return;
        start.current = null;
        setDragging(false);
        onCommit(latest.current);
      }}
      onDoubleClick={() => {
        onChange(LIST_DEFAULT);
        onCommit(LIST_DEFAULT);
      }}
      onKeyDown={(e) => {
        const next = keyWidth(latest.current, e.key, e.shiftKey);
        if (next === null) return;
        e.preventDefault();
        onChange(next);
        onCommit(next);
      }}
    />
  );
}

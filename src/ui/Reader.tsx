// DS: leitura.html · .reader, .reader__head, .reader__subject, .message, .message__head, .message__from, .message__addr, .message__time
// DS: espacamento.html · .app-shell__back (visível só abaixo de 960px)
import type { ReactNode } from "react";
import { Avatar } from "./Avatar";
import { IconButton } from "./Button";

interface ReaderProps {
  subject: string;
  fromName: string;
  fromAddress: string;
  time: string;
  onBack?: () => void;
  children?: ReactNode;
}

export function Reader({ subject, fromName, fromAddress, time, onBack, children }: ReaderProps) {
  return (
    <article className="reader" aria-label={subject}>
      <div className="reader__head">
        {onBack && <IconButton icon="chevron-left" label="Voltar para a lista" className="app-shell__back" onClick={onBack} />}
        <h1 className="reader__subject">{subject}</h1>
      </div>
      <div className="message">
        <div className="message__head">
          <Avatar name={fromName || fromAddress} seed={fromAddress} size="lg" />
          <div style={{ minWidth: 0 }}>
            <div className="message__from">
              {fromName || fromAddress}
              {fromName && fromAddress && <span className="message__addr"> &lt;{fromAddress}&gt;</span>}
            </div>
          </div>
          <div className="message__time">{time}</div>
        </div>
        {children}
      </div>
    </article>
  );
}

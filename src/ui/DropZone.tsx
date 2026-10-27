// DS: arquivos.html · .drop-zone--overlay (L70): por cima da janela enquanto um arquivo é arrastado sobre ela (US15)
import { Icon } from "./Icon";

export function DropZone() {
  return (
    <div className="drop-zone drop-zone--overlay" role="status">
      <Icon name="upload" />
      <p className="drop-zone__title">Solte para abrir no EpistulaMail</p>
      <p className="drop-zone__desc">.mbox, .pst, .msg, .eml, .emlx, ou uma pasta do Thunderbird, do Apple Mail ou Maildir.</p>
    </div>
  );
}

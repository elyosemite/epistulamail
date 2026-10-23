// DS: anexos.html · .attachments, .attachment.attachment--static ("Somente informação"),
//     .attachment__icon--pdf/--img/--doc, .attachment__name, .attachment__meta
import { attachmentKind, formatBytes } from "../lib/format";
import type { AttachmentInfo } from "../lib/tauri";
import { Icon } from "./Icon";

const TYPE_LABEL: Record<ReturnType<typeof attachmentKind>, string> = { pdf: "PDF", img: "Imagem", doc: "Documento", other: "Arquivo" };

export function AttachmentList({ attachments }: { attachments: AttachmentInfo[] }) {
  if (attachments.length === 0) return null;
  return (
    <div className="attachments" role="list" aria-label="Anexos">
      {attachments.map((a, i) => {
        const kind = attachmentKind(a.mimeType);
        const iconCls = ["attachment__icon", kind !== "other" && `attachment__icon--${kind}`].filter(Boolean).join(" ");
        return (
          <div key={`${a.name}-${i}`} className="attachment attachment--static" role="listitem" title="Download disponível numa próxima versão">
            <span className={iconCls}><Icon name={kind === "img" ? "image" : "file"} size="lg" /></span>
            <span style={{ minWidth: 0, display: "grid" }}>
              <span className="attachment__name">{a.name}</span>
              <span className="attachment__meta">{TYPE_LABEL[kind]} · {formatBytes(a.size)}</span>
            </span>
          </div>
        );
      })}
    </div>
  );
}

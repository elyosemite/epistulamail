// DS: modais.html "Link que leva a outro lugar" · .modal--warning com .link-check (L55): o que o texto mostra e o que
// abre, com só o domínio de destino em <mark>. O foco abre em "Cancelar".
import type { MisleadingLink } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { Modal } from "../../ui/Modal";

/** Divide o endereço em antes, domínio de destino e depois, para destacar só o domínio. */
export function splitHost(target: string, host: string): [string, string, string] {
  const at = target.indexOf(host);
  if (at < 0 || !host) return [target, "", ""];
  return [target.slice(0, at), host, target.slice(at + host.length)];
}

/** Link cujo texto mostra um endereço e abre outro (US10, CA8). */
export function LinkCheckDialog({ link, onOpen, onClose }: { link: MisleadingLink; onOpen: () => void; onClose: () => void }) {
  const [before, host, after] = splitHost(link.target, link.targetHost);
  return (
    <Modal
      title="Este link leva a outro endereço"
      icon="warning"
      appearance="warning"
      initialFocus="[data-cancel]"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" data-cancel onClick={onClose}>Cancelar</Button>
          <Button appearance="warning" onClick={onOpen}>Abrir mesmo assim</Button>
        </>
      }
    >
      <p>O texto mostra um endereço, mas o link abre outro. Golpes costumam usar esse truque.</p>
      <dl className="link-check">
        <div><dt>Mostra</dt><dd>{link.shown}</dd></div>
        <div><dt>Abre</dt><dd>{before}<mark>{host}</mark>{after}</dd></div>
      </dl>
    </Modal>
  );
}

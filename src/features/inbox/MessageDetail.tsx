// Composição de DS: leitura.html (.reader) + mensagens.html (.section-message) + vazios.html (.empty)
import { formatFullDate } from "../../lib/format";
import type { MessageHeader } from "../../lib/tauri";
import { EmptyState } from "../../ui/EmptyState";
import { Reader } from "../../ui/Reader";
import { SectionMessage } from "../../ui/SectionMessage";

export function MessageDetail({ message, onBack }: { message: MessageHeader | null; onBack: () => void }) {
  if (!message) {
    return <EmptyState title="Nenhuma mensagem aberta" description="Escolha uma mensagem na lista para ver os detalhes." />;
  }
  return (
    <Reader
      subject={message.subject || "(sem assunto)"}
      fromName={message.fromName}
      fromAddress={message.fromAddress}
      time={formatFullDate(message.date)}
      onBack={onBack}
    >
      <SectionMessage title="Conteúdo ainda não disponível">
        Nesta versão o Epistula carrega só os cabeçalhos. O corpo da mensagem chega numa próxima atualização.
      </SectionMessage>
    </Reader>
  );
}

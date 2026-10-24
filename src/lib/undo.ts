/**
 * Ação com "Desfazer" (US4, CA4): a mudança aparece na hora, mas o servidor só é chamado quando o
 * prazo acaba. Desfazer cancela sem chamar o servidor. Só uma ação fica pendente: uma nova
 * executa a anterior na hora.
 */
export const UNDO_MS = 6000;

export interface PendingAction {
  /** Executa no servidor. */
  run: () => Promise<void>;
  /** Volta a interface ao estado anterior (desfazer ou falha). */
  restore: () => void;
}

export class UndoQueue {
  private pending: { action: PendingAction; timer: ReturnType<typeof setTimeout> } | null = null;

  constructor(private readonly delay = UNDO_MS, private readonly onError: (action: PendingAction, error: unknown) => void = () => undefined) {}

  schedule(action: PendingAction): void {
    this.flush();
    const timer = setTimeout(() => this.execute(), this.delay);
    this.pending = { action, timer };
  }

  /** Desfaz a ação pendente. Devolve `false` se não havia nenhuma. */
  undo(): boolean {
    if (!this.pending) return false;
    clearTimeout(this.pending.timer);
    const { action } = this.pending;
    this.pending = null;
    action.restore();
    return true;
  }

  /** Executa agora a ação pendente (nova ação, troca de conta ou fechamento). */
  flush(): void {
    if (!this.pending) return;
    clearTimeout(this.pending.timer);
    this.execute();
  }

  private execute(): void {
    if (!this.pending) return;
    const { action } = this.pending;
    this.pending = null;
    action.run().catch((error) => {
      action.restore();
      this.onError(action, error);
    });
  }
}

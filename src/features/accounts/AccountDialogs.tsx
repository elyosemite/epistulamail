// Composição de DS: modais.html (.modal, .modal--warning), campos.html (.field), selecao.html (.chip-group,
// .color-picker), botoes.html (.btn--primary, .btn--warning, .btn--subtle)
import { useState } from "react";
import { ACCOUNT_COLORS, MAX_ACCOUNT_NAME, NAME_SUGGESTIONS, accountInitial } from "../../lib/accounts";
import type { Account, AccountColor } from "../../lib/tauri";
import { Button } from "../../ui/Button";
import { ColorPicker } from "../../ui/ColorPicker";
import { ConfirmCheck } from "../../ui/ConfirmCheck";
import { ChipGroup, TextField } from "../../ui/Field";
import { Modal } from "../../ui/Modal";

interface AccountSettingsProps {
  account: Account;
  /** Conta recém-conectada: título de boas-vindas e só "Concluir". */
  isNew: boolean;
  onSave: (change: { displayName: string; color: AccountColor }) => Promise<void>;
  onClose: () => void;
}

/** Nome e cor da conta (CA4): logo depois do login e, depois, por "Renomear" e "Mudar cor". */
export function AccountSettingsDialog({ account, isNew, onSave, onClose }: AccountSettingsProps) {
  const [name, setName] = useState(account.displayName);
  const [color, setColor] = useState<AccountColor>(account.color);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | undefined>();
  const trimmed = name.trim();

  const save = async () => {
    if (!trimmed) {
      setError("Dê um nome para a conta.");
      return;
    }
    setSaving(true);
    try {
      await onSave({ displayName: trimmed, color });
      onClose();
    } catch {
      setError("Não foi possível salvar. Tente de novo.");
      setSaving(false);
    }
  };

  return (
    <Modal
      title={isNew ? "Como quer chamar esta conta?" : "Nome e cor da conta"}
      initialFocus="input"
      onClose={onClose}
      footer={
        <>
          {!isNew && <Button appearance="subtle" onClick={onClose}>Cancelar</Button>}
          <Button appearance="primary" loading={saving} onClick={save}>{isNew ? "Concluir" : "Salvar"}</Button>
        </>
      }
    >
      <form
        style={{ display: "grid", gap: "var(--space-250)" }}
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        <p style={{ margin: 0, color: "var(--color-text-subtle)" }}>{account.email}</p>
        <TextField
          label="Nome da conta"
          value={name}
          maxLength={MAX_ACCOUNT_NAME}
          error={error}
          onChange={(e) => {
            setName(e.target.value);
            setError(undefined);
          }}
        >
          <ChipGroup label="Sugestões" options={NAME_SUGGESTIONS} value={trimmed} onPick={setName} />
        </TextField>
        <ColorPicker legend="Cor" options={ACCOUNT_COLORS} value={color} sample={accountInitial(trimmed)} onChange={setColor} />
      </form>
    </Modal>
  );
}

/** Sair da conta (CA6): a conta sai da coluna, mas os dados ficam no computador, por isso aviso e não perigo. Marcar
 * "Apagar também" (US10, CA7) apaga os dados da conta deste computador e o botão vira perigo. */
export function SignOutDialog({ account, onConfirm, onClose }: { account: Account; onConfirm: (erase: boolean) => Promise<void>; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  const [erase, setErase] = useState(false);
  return (
    <Modal
      title={`Sair de ${account.displayName}?`}
      icon="log-out"
      appearance="warning"
      initialFocus="[data-cancel]"
      onClose={onClose}
      footer={
        <>
          <Button appearance="subtle" data-cancel onClick={onClose}>Cancelar</Button>
          <Button
            appearance={erase ? "danger" : "warning"}
            loading={busy}
            onClick={async () => {
              setBusy(true);
              try {
                await onConfirm(erase);
              } finally {
                onClose();
              }
            }}
          >
            {erase ? "Sair e apagar" : "Sair"}
          </Button>
        </>
      }
    >
      <p>O Epistula deixa de acessar <b>{account.email}</b> e a conta sai da coluna de contas. As mensagens já salvas continuam neste computador, cifradas, e voltam quando você entrar de novo com o mesmo e-mail.</p>
      <p>Nada muda na sua conta de e-mail.</p>
      <ConfirmCheck checked={erase} onChange={setErase}>Apagar também os e-mails desta conta guardados neste computador.</ConfirmCheck>
    </Modal>
  );
}

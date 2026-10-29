import type { ManualAccountInput } from "./tauri";

export interface ManualFormState {
  incomingHost: string;
  incomingPort: string;
  incomingSecurity: "tls" | "starttls";
  incomingAuth: "password";
  incomingUser: string;
  incomingPassword: string;
  outgoingHost: string;
  outgoingPort: string;
  outgoingSecurity: "tls" | "starttls";
  outgoingAuth: "password";
  outgoingUser: string;
  outgoingPassword: string;
  sameCredentials: boolean;
}

export type ManualFieldErrors = Partial<Record<keyof ManualFormState, string>>;

export const INITIAL_MANUAL_FORM: ManualFormState = {
  incomingHost: "",
  incomingPort: "993",
  incomingSecurity: "tls",
  incomingAuth: "password",
  incomingUser: "",
  incomingPassword: "",
  outgoingHost: "",
  outgoingPort: "465",
  outgoingSecurity: "tls",
  outgoingAuth: "password",
  outgoingUser: "",
  outgoingPassword: "",
  sameCredentials: true,
};

export function portOf(value: string): number | null {
  const port = Number(value);
  return Number.isInteger(port) && port >= 1 && port <= 65535 ? port : null;
}

export function validHost(value: string): boolean {
  const host = value.trim();
  return host.length > 0 && host.length <= 253 && !/[\s/\\?#@]/.test(host) && !host.includes("://");
}

export function validateManualForm(form: ManualFormState): ManualFieldErrors {
  const errors: ManualFieldErrors = {};
  if (!validHost(form.incomingHost)) errors.incomingHost = "Informe o servidor de entrada, sem https:// nem caminho.";
  if (portOf(form.incomingPort) === null) errors.incomingPort = "Use uma porta entre 1 e 65535.";
  if (!form.incomingUser.includes("@") || /\s/.test(form.incomingUser)) errors.incomingUser = "Informe o e-mail completo, como ana@exemplo.com.";
  if (!form.incomingPassword) errors.incomingPassword = "Informe a senha da entrada.";
  if (!validHost(form.outgoingHost)) errors.outgoingHost = "Informe o servidor de saída, sem https:// nem caminho.";
  if (portOf(form.outgoingPort) === null) errors.outgoingPort = "Use uma porta entre 1 e 65535.";
  if (!form.sameCredentials) {
    if (!form.outgoingUser.includes("@") || /\s/.test(form.outgoingUser)) errors.outgoingUser = "Informe o e-mail completo da saída.";
    if (!form.outgoingPassword) errors.outgoingPassword = "Informe a senha da saída.";
  }
  return errors;
}

export function manualAccountInput(form: ManualFormState): ManualAccountInput {
  const incoming = {
    host: form.incomingHost.trim(),
    port: portOf(form.incomingPort) ?? 0,
    security: form.incomingSecurity,
    auth: form.incomingAuth,
    username: form.incomingUser.trim(),
    password: form.incomingPassword,
  };
  const outgoing = form.sameCredentials
    ? { ...incoming }
    : {
        host: form.outgoingHost.trim(),
        port: portOf(form.outgoingPort) ?? 0,
        security: form.outgoingSecurity,
        auth: form.outgoingAuth,
        username: form.outgoingUser.trim(),
        password: form.outgoingPassword,
      };
  return { incoming, outgoing, useIncomingCredentials: form.sameCredentials };
}

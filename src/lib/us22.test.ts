import { describe, expect, it } from "vitest";
import { INITIAL_MANUAL_FORM, manualAccountInput, portOf, validHost, validateManualForm, type ManualFormState } from "./manualAccount";

function form(change: Partial<ManualFormState> = {}): ManualFormState {
  return {
    ...INITIAL_MANUAL_FORM,
    incomingHost: "mail.exemplo.com",
    incomingUser: "ana@exemplo.com",
    incomingPassword: "segredo",
    outgoingHost: "smtp.exemplo.com",
    outgoingUser: "ana@exemplo.com",
    outgoingPassword: "segredo",
    ...change,
  };
}

describe("US22 · conta manual", () => {
  it("aceita uma configuração completa", () => {
    expect(validateManualForm(form())).toEqual({});
  });

  it("aponta os campos obrigatórios", () => {
    const errors = validateManualForm(form({ incomingHost: "", incomingUser: "ana", incomingPassword: "" }));
    expect(errors.incomingHost).toBeTruthy();
    expect(errors.incomingUser).toBeTruthy();
    expect(errors.incomingPassword).toBeTruthy();
  });

  it("exige credenciais próprias quando não são as mesmas da entrada", () => {
    const errors = validateManualForm(form({ sameCredentials: false, outgoingUser: "", outgoingPassword: "" }));
    expect(errors.outgoingUser).toBeTruthy();
    expect(errors.outgoingPassword).toBeTruthy();
  });

  it("copia entrada para saída quando as credenciais são compartilhadas", () => {
    const input = manualAccountInput(form({ sameCredentials: true }));
    expect(input.useIncomingCredentials).toBe(true);
    expect(input.outgoing).toEqual(input.incoming);
  });

  it("mantém a saída separada quando as credenciais são próprias", () => {
    const input = manualAccountInput(form({ sameCredentials: false, outgoingUser: "bia@exemplo.com", outgoingPassword: "outra" }));
    expect(input.useIncomingCredentials).toBe(false);
    expect(input.outgoing.username).toBe("bia@exemplo.com");
    expect(input.outgoing.password).toBe("outra");
    expect(input.outgoing.host).toBe("smtp.exemplo.com");
  });

  it("valida host, porta e esquema", () => {
    expect(validHost("mail.exemplo.com")).toBe(true);
    expect(validHost("https://mail.exemplo.com")).toBe(false);
    expect(validHost("mail.exemplo.com/path")).toBe(false);
    expect(validHost("mail exemplo.com")).toBe(false);
    expect(portOf("993")).toBe(993);
    expect(portOf("0")).toBeNull();
    expect(portOf("65536")).toBeNull();
    expect(portOf("abc")).toBeNull();
  });
});

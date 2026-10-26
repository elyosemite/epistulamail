import { describe, expect, it } from "vitest";
import { idleLimitMs, isLockShortcut, LOCK_CHOICES, mismatch, retryLabel, tooShort } from "./lock";

describe("US10 · senha do Epistula e bloqueio", () => {
  it("bloqueia depois do tempo escolhido, ou nunca", () => {
    expect(idleLimitMs("15")).toBe(15 * 60_000);
    expect(idleLimitMs("60")).toBe(60 * 60_000);
    expect(idleLimitMs("never")).toBeNull();
    expect(LOCK_CHOICES.map((c) => c.value)).toEqual(["5", "15", "30", "60", "never"]);
  });

  it("exige 8 caracteres e as duas senhas iguais", () => {
    expect(tooShort("curta")).toBe("Use pelo menos 8 caracteres.");
    expect(tooShort("frase longa")).toBeUndefined();
    // Conta caracteres, não unidades UTF-16.
    expect(tooShort("😀😀😀😀😀😀😀")).toBeDefined();
    expect(mismatch("frase longa", "frase longo")).toBe("As duas senhas não são iguais.");
    expect(mismatch("frase longa", "frase longa")).toBeUndefined();
  });

  it("diz quanto falta para tentar de novo", () => {
    expect(retryLabel(0)).toBe("Senha incorreta.");
    expect(retryLabel(1)).toBe("Senha incorreta. Tente de novo em 1 segundo.");
    expect(retryLabel(8)).toBe("Senha incorreta. Tente de novo em 8 segundos.");
    expect(retryLabel(61)).toBe("Senha incorreta. Tente de novo em 2 minutos.");
  });

  it("Ctrl+Shift+L bloqueia; Ctrl+L sozinho não", () => {
    const key = (k: Partial<{ ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean; key: string }>) => ({
      ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, key: "l", ...k,
    });
    expect(isLockShortcut(key({ ctrlKey: true, shiftKey: true, key: "L" }))).toBe(true);
    expect(isLockShortcut(key({ ctrlKey: true }))).toBe(false);
    expect(isLockShortcut(key({ ctrlKey: true, shiftKey: true, altKey: true }))).toBe(false);
  });
});

describe("US10 · link que leva a outro endereço", () => {
  it("destaca só o domínio de destino", async () => {
    const { splitHost } = await import("../features/message/LinkCheckDialog");
    expect(splitHost("https://meubanco-seguro.xyz/entrar", "meubanco-seguro.xyz")).toEqual(["https://", "meubanco-seguro.xyz", "/entrar"]);
    expect(splitHost("https://x.com/", "")).toEqual(["https://x.com/", "", ""]);
  });
});

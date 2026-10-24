import { describe, expect, it } from "vitest";
import { accountInitial, accountShortcut, avatarColorClass, pickActive, railLabel, railTip, unreadBadge } from "./accounts";
import type { Account } from "./tauri";

const account = (over: Partial<Account> = {}): Account => ({
  id: "a1",
  provider: "gmail",
  email: "joao.silva.empresa@gmail.com",
  displayName: "Trabalho",
  color: "purple",
  position: 0,
  status: "active",
  unread: 12,
  lastFolderId: null,
  listWidth: null,
  ...over,
});

const key = (k: string, mods: Partial<Record<"ctrlKey" | "metaKey" | "altKey" | "shiftKey", boolean>> = {}) => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe("contas", () => {
  it("usa a inicial do nome da conta, não do e-mail", () => {
    expect(accountInitial("trabalho")).toBe("T");
    expect(accountInitial("  família")).toBe("F");
    expect(accountInitial("Écio")).toBe("É");
    expect(accountInitial("")).toBe("?");
  });

  it("limita o contador a 99+", () => {
    expect(unreadBadge(null)).toBeNull();
    expect(unreadBadge(0)).toBeNull();
    expect(unreadBadge(12)).toBe("12");
    expect(unreadBadge(99)).toBe("99");
    expect(unreadBadge(100)).toBe("99+");
  });

  it("monta o nome acessível e o tooltip", () => {
    expect(railLabel(account())).toBe("Trabalho, joao.silva.empresa@gmail.com, 12 não lidas");
    expect(railLabel(account({ unread: 150 }))).toBe("Trabalho, joao.silva.empresa@gmail.com, mais de 99 não lidas");
    expect(railLabel(account({ status: "signed_out" }))).toBe("Trabalho, joao.silva.empresa@gmail.com, você saiu desta conta");
    expect(railLabel(account({ status: "needs_reauth" }))).toBe("Trabalho, joao.silva.empresa@gmail.com, precisa reconectar");
    expect(railTip(account(), 1)).toBe("Trabalho · joao.silva.empresa@gmail.com (Ctrl+2)");
    expect(railTip(account(), 9)).toBe("Trabalho · joao.silva.empresa@gmail.com");
  });

  it("reconhece Ctrl+1 a Ctrl+9", () => {
    expect(accountShortcut(key("1", { ctrlKey: true }))).toBe(0);
    expect(accountShortcut(key("9", { ctrlKey: true }))).toBe(8);
    expect(accountShortcut(key("0", { ctrlKey: true }))).toBeNull();
    expect(accountShortcut(key("1"))).toBeNull();
    expect(accountShortcut(key("1", { ctrlKey: true, shiftKey: true }))).toBeNull();
    expect(accountShortcut(key("a", { ctrlKey: true }))).toBeNull();
  });

  it("azul é a cor padrão do avatar", () => {
    expect(avatarColorClass("blue")).toBeNull();
    expect(avatarColorClass("green")).toBe("avatar--green");
  });

  it("mantém a conta ativa se ela ainda existir", () => {
    const list = [account({ id: "a" }), account({ id: "b" })];
    expect(pickActive(list, "b")).toBe("b");
    expect(pickActive(list, "sumiu")).toBe("a");
    expect(pickActive([], "a")).toBeNull();
  });
});

import { describe, expect, it } from "vitest";
import { decodeChoice, encodeChoice, intervalLabel, intervalShort, SYNC_CHOICES } from "./syncInterval";

describe("intervalo de sincronização", () => {
  it("rótulos", () => {
    expect(SYNC_CHOICES.map(intervalLabel)).toEqual(["Manualmente", "1 minuto", "2 minutos", "5 minutos", "10 minutos", "15 minutos", "30 minutos", "1 hora"]);
    expect(intervalShort(5)).toBe("5 min");
    expect(intervalShort(60)).toBe("1 h");
    expect(intervalShort(null)).toBe("manual");
  });

  it("ida e volta pelo valor do select", () => {
    for (const m of SYNC_CHOICES) expect(decodeChoice(encodeChoice(m))).toEqual({ useDefault: false, minutes: m });
    expect(decodeChoice("default")).toEqual({ useDefault: true });
    expect(() => decodeChoice("3")).toThrow();
    expect(() => decodeChoice("abc")).toThrow();
  });
});

import { describe, expect, it } from "vitest";
import type { GameConfig } from "@/lib/types";
import { configToForm, formToConfig } from "./configForm";

const DEFAULT: GameConfig = {
  schema_version: 1,
  players: 6,
  small_blind: 500,
  ante: 0,
  stack: { effective_stack: 100_000 },
  rake: {
    percentage_bps: 500,
    cap: null,
    min_pot: null,
    rounding: { increment: { unit: "big_blinds", mbb: 1 }, mode: "down" },
    no_flop_no_drop: true,
    allocation: "proportional",
  },
  stake: null,
};

describe("game config form", () => {
  it("round-trips the engine default exactly", () => {
    const r = formToConfig(configToForm(DEFAULT));
    expect(r).toEqual({ ok: true, config: DEFAULT });
  });

  it("converts custom values exactly", () => {
    const f = configToForm(DEFAULT);
    const r = formToConfig({
      ...f,
      stackBb: "62.5",
      rakePercent: "4.75",
      cap: { enabled: true, unit: "currency", value: "300" },
      stakeEnabled: true,
      stakeCurrency: "EUR",
      stakeBigBlindMinor: "50",
    });
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.config.stack.effective_stack).toBe(62_500);
      expect(r.config.rake.percentage_bps).toBe(475);
      expect(r.config.rake.cap).toEqual({ unit: "currency", minor_units: 300 });
      expect(r.config.stake).toEqual({ currency: "EUR", big_blind_minor_units: 50 });
    }
  });

  it("reports field errors instead of rounding", () => {
    const f = configToForm(DEFAULT);
    const r = formToConfig({
      ...f,
      stackBb: "100.0005",
      rakePercent: "x",
      cap: { enabled: true, unit: "currency", value: "1.5" },
    });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(Object.keys(r.errors).sort()).toEqual(["cap", "rakePercent", "stackBb"]);
  });
});

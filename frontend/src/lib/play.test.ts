import { describe, expect, it } from "vitest";
import { DEFAULT_PLAY_FORM, amount, buildTableConfig, parseAmount, parseLevels } from "./play";
import type { GameConfig } from "./types";

const GAME = { schema_version: 1, players: 6, stack: { effective_stack: 100_000 } } as unknown as GameConfig;

describe("play table config", () => {
  it("builds a cash table with mixed bots and an exact stack", () => {
    const r = buildTableConfig({ ...DEFAULT_PLAY_FORM, seats: "4", stackBb: "62.5" }, GAME, 9);
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    const c = r.config as {
      format: { type: string; game: { players: number; stack: { effective_stack: number } }; auto_rebuy: boolean };
      opponents: string[];
      seed: number;
    };
    expect(c.format.type).toBe("cash");
    expect(c.format.game.players).toBe(4);
    expect(c.format.game.stack.effective_stack).toBe(62_500);
    expect(c.opponents).toEqual(["easy", "medium", "hard"]);
    expect(c.seed).toBe(9);
  });

  it("builds a sit & go from level text and rejects bad levels", () => {
    expect(parseLevels("10/20, 15/30/5")).toEqual([
      { small_blind: 10, big_blind: 20, ante: 0 },
      { small_blind: 15, big_blind: 30, ante: 5 },
    ]);
    expect(parseLevels("10")).toBeNull();
    expect(parseLevels("10/20.5")).toBeNull();
    const r = buildTableConfig({ ...DEFAULT_PLAY_FORM, format: "sit_and_go", seats: "2", difficulty: "hard" }, GAME, 1);
    expect(r.ok && (r.config.format as { type: string }).type).toBe("sit_and_go");
    const bad = buildTableConfig({ ...DEFAULT_PLAY_FORM, format: "sit_and_go", levels: "x" }, GAME, 1);
    expect(bad.ok).toBe(false);
    expect(r.ok && (r.config.format as { payouts: number[] }).payouts).toEqual([]);
    const paid = buildTableConfig({ ...DEFAULT_PLAY_FORM, format: "sit_and_go", payouts: "50/30/20" }, GAME, 1);
    expect(paid.ok && (paid.config.format as { payouts: number[] }).payouts).toEqual([50, 30, 20]);
    const inexact = buildTableConfig({ ...DEFAULT_PLAY_FORM, format: "sit_and_go", payouts: "65.5/34.5" }, GAME, 1);
    expect(!inexact.ok && inexact.errors.payouts).toBeTruthy();
  });

  it("formats and parses amounts in table units without rounding", () => {
    expect(amount(2500, "bb")).toBe("2.5bb");
    expect(amount(1500, "chips")).toBe("1,500");
    expect(parseAmount("7.25", "bb")).toBe(7250);
    expect(parseAmount("7.2505", "bb")).toBeNull();
    expect(parseAmount("450", "chips")).toBe(450);
    expect(parseAmount("4.5", "chips")).toBeNull();
  });
});

import { describe, expect, it } from "vitest";
import { DEFAULT_PREFLOP_FORM, buildPreflopSpec, pushFoldForm } from "./preflop";
import type { GameConfig } from "./types";

const GAME = { schema_version: 1, players: 6, stack: { effective_stack: 100_000 } } as unknown as GameConfig;

type Spec = {
  game: { players: number; stack: { effective_stack: number } };
  heads_up: boolean;
  sizing: { open_raise_to_mbb: number[]; reraise_pct_of_bet: number[]; max_raises: number; add_all_in: boolean };
  max_seconds: number | null;
  check_interval: number;
};

describe("preflop spec builder", () => {
  it("converts sizes exactly", () => {
    const r = buildPreflopSpec(
      { ...DEFAULT_PREFLOP_FORM, players: "4", stackBb: "40.5", opens: "2, 2.25", reraises: "3x, 2.5" },
      GAME,
    );
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    const spec = r.spec as Spec;
    expect(spec.game.players).toBe(4);
    expect(spec.heads_up).toBe(false);
    expect(spec.game.stack.effective_stack).toBe(40_500);
    expect(spec.sizing.open_raise_to_mbb).toEqual([2000, 2250]);
    expect(spec.sizing.reraise_pct_of_bet).toEqual([300, 250]);
    expect(spec.max_seconds).toBe(1200);
    expect(spec.check_interval).toBe(30);
  });

  it("builds heads-up push/fold without changing the game's player count", () => {
    const r = buildPreflopSpec(pushFoldForm("10"), GAME);
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    const spec = r.spec as Spec;
    expect(spec.heads_up).toBe(true);
    expect(spec.game.players).toBe(6);
    expect(spec.sizing).toMatchObject({ open_raise_to_mbb: [], max_raises: 1, add_all_in: true });
  });

  it("rejects inexact or out-of-range input instead of rounding it", () => {
    const r = buildPreflopSpec(
      { ...DEFAULT_PREFLOP_FORM, players: "7", opens: "2.0001", reraises: "0", maxRaises: "0", maxMinutes: "1.5" },
      GAME,
    );
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(Object.keys(r.errors).sort()).toEqual(["maxMinutes", "maxRaises", "opens", "players", "reraises"]);
  });
});

import { describe, expect, it } from "vitest";
import { DEFAULT_FORM, buildSolveSpec, parseSizes } from "./solve";
import type { GameConfig } from "./types";

const GAME = { schema_version: 1, players: 6 } as unknown as GameConfig;

describe("solve spec builder", () => {
  it("parses whole-percentage size lists", () => {
    expect(parseSizes("33, 75")).toEqual([33, 75]);
    expect(parseSizes("33% 150%")).toEqual([33, 150]);
    expect(parseSizes("")).toEqual([]);
    expect(parseSizes("33.5")).toBeNull();
    expect(parseSizes("0")).toBeNull();
  });

  it("converts bb exactly and clears streets before the board's street", () => {
    const r = buildSolveSpec({ ...DEFAULT_FORM, board: "Ks7d2c8h", potBb: "6.5", stackBb: "97.5" }, GAME);
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    const spec = r.spec as {
      board: string[];
      starting_pot_mbb: number;
      effective_stack_mbb: number;
      tree: { ip: { bet_pct_pot: number[][] } };
      solver: { stopping: { target_exploitability_pct_pot: number } };
      game: unknown;
    };
    expect(spec.board).toEqual(["Ks", "7d", "2c", "8h"]);
    expect(spec.starting_pot_mbb).toBe(6500);
    expect(spec.effective_stack_mbb).toBe(97500);
    expect(spec.tree.ip.bet_pct_pot).toEqual([[], [75], [75]]);
    expect(spec.solver.stopping.target_exploitability_pct_pot).toBe(0.5);
    expect(spec.game).toBe(GAME);
  });

  it("reports field errors instead of guessing", () => {
    const r = buildSolveSpec(
      {
        ...DEFAULT_FORM,
        board: "Ks7",
        potBb: "1.0005",
        oop: { ...DEFAULT_FORM.oop, bet: { ...DEFAULT_FORM.oop.bet, flop: "33.3" } },
      },
      GAME,
    );
    expect(r.ok).toBe(false);
    if (!r.ok) expect(Object.keys(r.errors).sort()).toEqual(["board", "oop.bet.flop", "potBb"]);
  });
});

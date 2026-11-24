import { describe, expect, it } from "vitest";
import { DEFAULT_FORM, type SolveSpecIn, buildSolveSpec, formFromSpec, matchAction, parseSizes } from "./solve";
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

describe("spots from played hands", () => {
  it("fills the form from a spec and rebuilds the same tree", () => {
    const r = buildSolveSpec({ ...DEFAULT_FORM, board: "Ks7d2c8h" }, GAME);
    if (!r.ok) throw new Error("default form is valid");
    const spec = r.spec as unknown as SolveSpecIn;
    const form = formFromSpec(spec);
    expect(form.board).toBe("Ks 7d 2c 8h");
    expect(form.potBb).toBe("6.5");
    expect(form.ip.bet.turn).toBe("75");
    expect(form.ip.bet.flop).toBe("");
    const again = buildSolveSpec(form, GAME);
    expect(again.ok && again.spec.tree).toEqual(r.spec.tree);
    expect(again.ok && again.spec.starting_pot_mbb).toBe(6500);
  });

  it("matches a real action to the nearest solver action of the same kind", () => {
    const labels = ["fold", "call 3.8bb", "raise to 9bb", "raise to 20bb"];
    expect(matchAction(labels, { player: 0, kind: "raise", to_mbb: 11_000, all_in: false })).toBe(2);
    expect(matchAction(labels, { player: 0, kind: "raise", to_mbb: 18_000, all_in: false })).toBe(3);
    expect(matchAction(labels, { player: 0, kind: "call", to_mbb: 3750, all_in: false })).toBe(1);
    expect(matchAction(labels, { player: 0, kind: "check", to_mbb: 0, all_in: false })).toBeNull();
  });
});

describe("node lock helpers", () => {
  it("reads percentages that add up to 100 and rejects anything else", async () => {
    const { lockFrequencies } = await import("./solve");
    expect(lockFrequencies(["25", "75"])).toEqual({ ok: true, value: [0.25, 0.75] });
    expect(lockFrequencies(["33.3", "33.3", "33.4"]).ok).toBe(true);
    expect(lockFrequencies(["50", "60"])).toEqual({ ok: false, error: "The percentages add up to 110%, not 100%." });
    expect(lockFrequencies(["-1", "101"]).ok).toBe(false);
    expect(lockFrequencies(["", "100"]).ok).toBe(false);
  });

  it("adds a lock, replacing one at the same decision and keeping the others", async () => {
    const { withLock } = await import("./solve");
    const spec = {
      locks: [
        { path: [0], frequencies: [1, 0] },
        { path: [], frequencies: [0.5, 0.5] },
      ],
    };
    expect(withLock(spec, [0], [0.2, 0.8]).locks).toEqual([
      { path: [], frequencies: [0.5, 0.5] },
      { path: [0], frequencies: [0.2, 0.8] },
    ]);
    expect(withLock({} as { locks?: { path: number[]; frequencies: number[] }[] }, [1], [1, 0]).locks).toEqual([
      { path: [1], frequencies: [1, 0] },
    ]);
  });
});

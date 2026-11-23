/**
 * Builds the backend's SolveSpec from form input. Exact unit conversion only; every semantic
 * check (board, ranges, tree, rake) happens in the engine.
 */
import type { GameConfig } from "./types";
import { bbToMbb, parseScaled } from "./units";

export const STREETS = ["flop", "turn", "river"] as const;
export type Street = (typeof STREETS)[number];

export interface PlayerSizingForm {
  bet: Record<Street, string>;
  raise: Record<Street, string>;
}

export interface SolveForm {
  board: string;
  oopRange: string;
  ipRange: string;
  potBb: string;
  stackBb: string;
  oop: PlayerSizingForm;
  ip: PlayerSizingForm;
  maxRaises: string;
  allInThresholdPct: string;
  addAllIn: boolean;
  iterations: string;
  targetPct: string;
}

export type SpecResult = { ok: true; spec: Record<string, unknown> } | { ok: false; errors: Record<string, string> };

/** "33, 75" -> [33, 75]; whole percentages only. */
export function parseSizes(s: string): number[] | null {
  const parts = s
    .split(/[\s,]+/)
    .map((x) => x.trim())
    .filter(Boolean);
  const out: number[] = [];
  for (const p of parts) {
    const v = parseScaled(p.replace(/%$/, ""), 0);
    if (v === null || v < 1) return null;
    out.push(v);
  }
  return out;
}

export function boardCards(board: string): string[] {
  const clean = board.replace(/[\s,]/g, "");
  const out: string[] = [];
  for (let i = 0; i + 1 < clean.length; i += 2) out.push(clean.slice(i, i + 2));
  return out;
}

export function buildSolveSpec(form: SolveForm, game: GameConfig): SpecResult {
  const errors: Record<string, string> = {};
  const board = boardCards(form.board);
  if (![3, 4, 5].includes(board.length) || form.board.replace(/[\s,]/g, "").length % 2 !== 0) {
    errors.board = "Enter a flop, turn or river board, e.g. Ks 7d 2c";
  }
  const pot = bbToMbb(form.potBb);
  if (pot === null || pot === 0) errors.potBb = "Enter the starting pot in bb (up to 3 decimals)";
  const stack = bbToMbb(form.stackBb);
  if (stack === null || stack === 0) errors.stackBb = "Enter the effective stack in bb (up to 3 decimals)";
  const maxRaises = parseScaled(form.maxRaises, 0);
  if (maxRaises === null || maxRaises > 10) errors.maxRaises = "0–10";
  const threshold = parseScaled(form.allInThresholdPct, 0);
  if (threshold === null || threshold < 1 || threshold > 100) errors.allInThresholdPct = "1–100";
  const iterations = parseScaled(form.iterations, 0);
  if (iterations === null || iterations < 1) errors.iterations = "Enter a whole number of iterations";
  const target = form.targetPct.trim() === "" ? null : parseScaled(form.targetPct, 3);
  if (form.targetPct.trim() !== "" && (target === null || target === 0))
    errors.targetPct = "A positive % of the pot, or blank";

  // Streets before the board's street cannot be bet on.
  const firstStreet = Math.max(0, board.length - 3);
  const sizing = (p: PlayerSizingForm, who: string) => {
    const bet: number[][] = [];
    const raise: number[][] = [];
    STREETS.forEach((st, i) => {
      const b = parseSizes(p.bet[st]);
      const r = parseSizes(p.raise[st]);
      if (b === null) errors[`${who}.bet.${st}`] = "Whole percentages, e.g. 33, 75";
      if (r === null) errors[`${who}.raise.${st}`] = "Whole percentages, e.g. 100";
      bet.push(i < firstStreet ? [] : (b ?? []));
      raise.push(i < firstStreet ? [] : (r ?? []));
    });
    return { bet_pct_pot: bet, raise_pct_pot: raise };
  };
  const oop = sizing(form.oop, "oop");
  const ip = sizing(form.ip, "ip");
  if (Object.keys(errors).length > 0) return { ok: false, errors };

  const checkEvery = Math.max(10, Math.ceil((iterations as number) / 20));
  return {
    ok: true,
    spec: {
      game,
      ranges: [form.oopRange.trim(), form.ipRange.trim()],
      board,
      starting_pot_mbb: pot,
      effective_stack_mbb: stack,
      tree: {
        oop,
        ip,
        max_raises_per_street: maxRaises,
        all_in_threshold_pct_stack: threshold,
        add_all_in: form.addAllIn,
      },
      solver: {
        schema_version: 1,
        variant: { algorithm: "discounted", alpha: 1.5, beta: 0.0, gamma: 2.0 },
        averaging: "quadratic",
        stopping: {
          max_iterations: iterations,
          // Percentage with up to 3 decimals, converted back exactly.
          target_exploitability_pct_pot: target === null ? null : target / 1000,
          max_seconds: null,
          exploitability_check_interval: checkEvery,
        },
        threads: 1,
        seed: null,
      },
    },
  };
}

export const DEFAULT_FORM: SolveForm = {
  board: "Ks 7d 2c",
  oopRange: "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s",
  ipRange: "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+",
  potBb: "6.5",
  stackBb: "97.5",
  oop: { bet: { flop: "33", turn: "75", river: "75" }, raise: { flop: "", turn: "", river: "" } },
  ip: { bet: { flop: "33, 75", turn: "75", river: "75" }, raise: { flop: "100", turn: "", river: "" } },
  maxRaises: "1",
  allInThresholdPct: "85",
  addAllIn: false,
  iterations: "300",
  targetPct: "0.5",
};

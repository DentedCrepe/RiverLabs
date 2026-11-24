/**
 * Builds the backend's SolveSpec from form input. Exact unit conversion only; every semantic
 * check (board, ranges, tree, rake) happens in the engine.
 */
import type { GameConfig } from "./types";
import { bbToMbb, formatBb, parseScaled } from "./units";

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

/** The engine's SolveSpec, as far as the form needs it (e.g. a spot exported from a hand). */
export interface SolveSpecIn {
  game: GameConfig;
  ranges: string[];
  board: string[];
  starting_pot_mbb: number;
  effective_stack_mbb: number;
  tree: {
    oop: { bet_pct_pot: number[][]; raise_pct_pot: number[][] };
    ip: { bet_pct_pot: number[][]; raise_pct_pot: number[][] };
    max_raises_per_street: number;
    all_in_threshold_pct_stack: number;
    add_all_in: boolean;
  };
  solver: { stopping: { max_iterations: number; target_exploitability_pct_pot: number | null } };
}

/** Fills the form from a spec (the inverse of buildSolveSpec for the fields it uses). */
export function formFromSpec(spec: SolveSpecIn): SolveForm {
  const sizing = (p: SolveSpecIn["tree"]["oop"]): PlayerSizingForm => {
    const join = (v: number[][], i: number) => (v[i] ?? []).join(", ");
    return {
      bet: { flop: join(p.bet_pct_pot, 0), turn: join(p.bet_pct_pot, 1), river: join(p.bet_pct_pot, 2) },
      raise: { flop: join(p.raise_pct_pot, 0), turn: join(p.raise_pct_pot, 1), river: join(p.raise_pct_pot, 2) },
    };
  };
  const target = spec.solver.stopping.target_exploitability_pct_pot;
  return {
    board: spec.board.join(" "),
    oopRange: spec.ranges[0],
    ipRange: spec.ranges[1],
    potBb: formatBb(spec.starting_pot_mbb),
    stackBb: formatBb(spec.effective_stack_mbb),
    oop: sizing(spec.tree.oop),
    ip: sizing(spec.tree.ip),
    maxRaises: String(spec.tree.max_raises_per_street),
    allInThresholdPct: String(spec.tree.all_in_threshold_pct_stack),
    addAllIn: spec.tree.add_all_in,
    iterations: String(spec.solver.stopping.max_iterations),
    targetPct: target === null ? "" : String(target),
  };
}

/** One step of a line to follow in a solved tree (engine play_engine::spot::ExportStep). */
export interface LineStep {
  player: number;
  kind: string;
  to_mbb: number;
  all_in: boolean;
}

/**
 * The solver action at a node that matches a step of a real line: the same kind of action
 * ("fold", "check", "call", "bet", "raise"), and among those the nearest amount. Labels are the
 * engine's ("bet 6.6bb", "raise to 20bb"). `null` if no action of that kind exists.
 */
export function matchAction(labels: string[], step: LineStep): number | null {
  const kind = (l: string) => (l.startsWith("raise") ? "raise" : l.split(" ")[0]);
  const amount = (l: string) => {
    const m = /([\d.]+)bb$/.exec(l);
    return m ? Number(m[1]) * 1000 : null;
  };
  let best: number | null = null;
  let bestDist = Infinity;
  labels.forEach((l, i) => {
    if (kind(l) !== step.kind) return;
    const a = amount(l);
    const d = a === null ? 0 : Math.abs(a - step.to_mbb);
    if (d < bestDist) {
      best = i;
      bestDist = d;
    }
  });
  return best;
}

/**
 * Reads lock frequencies typed as percentages (one per action) and returns fractions that add up
 * to exactly 1, or an error. The percentages must add up to 100 (to within 0.01).
 */
export function lockFrequencies(percent: string[]): { ok: true; value: number[] } | { ok: false; error: string } {
  const values = percent.map((p) => Number(p.trim()));
  if (percent.some((p) => p.trim() === "") || values.some((v) => !Number.isFinite(v) || v < 0 || v > 100)) {
    return { ok: false, error: "Give each action a percentage between 0 and 100." };
  }
  const total = values.reduce((s, v) => s + v, 0);
  if (Math.abs(total - 100) > 0.01) {
    return { ok: false, error: `The percentages add up to ${Number(total.toFixed(2))}%, not 100%.` };
  }
  return { ok: true, value: values.map((v) => v / total) };
}

/** The spec with a lock at `path` (replacing any lock already there; other locks are kept). */
export function withLock<T extends { locks?: { path: number[]; frequencies: number[] }[] }>(
  spec: T,
  path: number[],
  frequencies: number[],
): T {
  const same = (p: number[]) => p.length === path.length && p.every((x, i) => x === path[i]);
  return { ...spec, locks: [...(spec.locks ?? []).filter((l) => !same(l.path)), { path, frequencies }] };
}

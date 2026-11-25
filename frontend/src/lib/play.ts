/**
 * Play against bots: request/response types and the table-config builder. Exact unit
 * conversion only; the engine validates every rule (seats, blinds, stacks).
 */
import type { LineStep, SolveSpecIn } from "./solve";
import type { ClassStrategy, GameConfig } from "./types";
import { bbToMbb, formatBb, parseScaled } from "./units";

export type Difficulty = "easy" | "medium" | "hard";
export type FormatType = "cash" | "fast_fold" | "sit_and_go";

export type Ratio = { made: number; opportunities: number };
export type SeatView = {
  name: string;
  is_hero: boolean;
  position: string | null;
  is_button: boolean;
  stack: number;
  street_commit: number;
  folded: boolean;
  all_in: boolean;
  busted: boolean;
  finish: number | null;
  cards: string[] | null;
  in_hand: boolean;
  last_action: string | null;
  hidden_cards: string[] | null;
  /** Sit & go: share of the prize pool by ICM (Malmuth–Harville), from the stacks at the start of the hand. */
  icm_share?: number | null;
};
export type Feedback = {
  verdict: "mistake" | "rare" | "ok" | "not_graded";
  headline: string;
  detail: string;
  reference: string | null;
};
export type TableView = {
  status: "awaiting_hero" | "hand_over" | "session_over";
  format: FormatType;
  units: "bb" | "chips";
  big_blind: number;
  small_blind: number;
  ante: number;
  level: number | null;
  hand_no: number;
  street: string | null;
  board: string[];
  pot: number;
  seats: SeatView[];
  to_act: number | null;
  legal: {
    fold: boolean;
    check: boolean;
    call: number | null;
    bet: [number, number] | null;
    raise: [number, number] | null;
  } | null;
  log: { street: string; player: string; text: string }[];
  last_hand_net: number | null;
  hero_total_net: number;
  notes: string[];
  reveal: boolean;
  feedback: { street: string; action: string; feedback: Feedback }[];
  summary: string[];
  /** Sit & go: percent of the prize pool for 1st, 2nd, ... */
  payouts?: number[] | null;
  hero_equity: number | null;
  required_equity: number | null;
};

export type RecordAction = {
  seat: number;
  street: string;
  action: { type: "fold" | "check" | "call" | "bet" | "raise"; to?: number };
  added: number;
  street_commit: number;
  all_in: boolean;
};
export type HeroDecision = {
  street: string;
  action: RecordAction["action"];
  to_call: number;
  pot: number;
  required_equity: number | null;
  equity_vs_ranges: number | null;
  opponents: number;
  model: { spot: string; class: string; actions: string[]; frequencies: number[]; taken: number | null } | null;
  all_in_call: boolean;
  feedback: Feedback | null;
  /** Heads-up after the flop: the decision can be solved (engine play_engine::spot::SpotInfo). */
  spot?: { street: string; board: string[] } | null;
  /** Sit & go all-in, heads-up: the call valued with ICM (shares of the remaining prize pool). */
  icm?: { payouts: number[]; fold: number; win: number; lose: number; required_equity: number } | null;
  /** Your equity at this decision and what it was made of, with each bot's real cards. */
  breakdown?: EquityBreakdown | null;
};
/** A solve built from one of your decisions (engine play_engine::spot::SpotExport). */
export type SpotExport = {
  spec: SolveSpecIn;
  line: LineStep[];
  hero_action: LineStep;
  hero_player: number;
  hero_cards: string[];
  hero_class: string;
  street: string;
  notes: string[];
};
/** A finished hand (engine play_engine::HandRecord). */
export type HandRecord = {
  hand_no: number;
  level: number | null;
  tournament: boolean;
  small_blind: number;
  big_blind: number;
  ante: number;
  seats: {
    name: string;
    position: string;
    starting_stack: number;
    hole_cards: string[];
    shown: boolean;
    is_hero: boolean;
  }[];
  board: string[];
  actions: RecordAction[];
  net: number[];
  showdown: boolean;
  hero_decisions: HeroDecision[];
  hero_all_in_ev: number | null;
  revealed: boolean;
  summary: string[];
};

/** "raises to 7.5bb" — another player's action, in the table's units. */
export function describeAction(a: RecordAction["action"], units: "bb" | "chips", allIn = false, you = false): string {
  const ai = allIn ? " (all-in)" : "";
  const s = (verb: string) => (you ? verb : `${verb}s`);
  switch (a.type) {
    case "fold":
      return s("fold");
    case "check":
      return s("check");
    case "call":
      return `${s("call")}${ai}`;
    case "bet":
      return `${s("bet")} ${amount(a.to ?? 0, units)}${ai}`;
    case "raise":
      return `${you ? "raise" : "raises"} to ${amount(a.to ?? 0, units)}${ai}`;
  }
}
export type PlaySessionInfo = {
  id: string;
  format: FormatType;
  status: "preparing" | "active" | "finished" | "interrupted" | "failed";
  error_message: string | null;
  hands: number;
  hero_net: number;
  created_at: string;
  finished_at: string | null;
  config: Record<string, unknown>;
  /** False for a game that does not count towards your statistics (ticked at the start, or guest mode). */
  tracked: boolean;
};
export type PlayState = { session: PlaySessionInfo; table: TableView | null };
export type Finding = {
  key: string;
  title: string;
  detail: string;
  measured_against: string;
  count: number;
  out_of: number;
  /** `session_id` is set in Analytics (hands from many games). */
  examples: { record: number; hand_no: number; street: string; text: string; session_id?: string }[];
};
export type PlayReport = {
  hands: number;
  units: "bb" | "chips";
  net: number;
  net_bb: number;
  bb_per_100: number | null;
  stats: {
    vpip: Ratio;
    pfr: Ratio;
    three_bet: Ratio;
    fold_to_three_bet: Ratio;
    cbet: Ratio;
    fold_to_cbet: Ratio;
    went_to_showdown: Ratio;
    won_at_showdown: Ratio;
    aggression_factor: number | null;
  };
  by_position: { position: string; hands: number; net_bb: number; vpip: Ratio }[];
  model_comparison: {
    decisions: number;
    voluntary_you: number;
    voluntary_model: number;
    raise_you: number;
    raise_model: number;
  } | null;
  all_in: { hands: number; actual_bb: number; expected_bb: number } | null;
  findings: Finding[];
  /** River play against bots given a practice leak (engine report::LeakPractice). */
  leak_practice?: LeakPractice[];
  caveats: string[];
};

export type Leak = "none" | "overbluffs_river" | "never_bluffs_river" | "overfolds_river" | "calls_too_much_river";

export const LEAKS: { value: Leak; label: string }[] = [
  { value: "none", label: "None" },
  { value: "overbluffs_river", label: "Over-bluffs the river" },
  { value: "never_bluffs_river", label: "Never bluffs the river" },
  { value: "overfolds_river", label: "Folds too much to river bets" },
  { value: "calls_too_much_river", label: "Calls too much on the river" },
];

export type LeakPractice = {
  leak: Leak;
  label: string;
  best_response: string;
  decisions: number;
  matched: number;
  examples: Finding["examples"];
  bets: number;
  mean_fold_share: number | null;
  mean_break_even: number | null;
  bets_profitable_as_bluffs: number;
};

export interface PlayForm {
  format: FormatType;
  seats: string;
  /** One difficulty for every bot, or "mixed" (easy, medium, hard in turn). */
  difficulty: Difficulty | "mixed";
  stackBb: string;
  autoRebuy: boolean;
  startingChips: string;
  /** "10/20, 15/30, 25/50/5": small/big[/ante] per level. */
  levels: string;
  handsPerLevel: string;
  /** A river leak given to every bot, to practise exploiting it. */
  leak: Leak;
  /** "65/35": percent of the prize pool per place; empty for the default for the table size. */
  payouts: string;
}

export const DEFAULT_PLAY_FORM: PlayForm = {
  format: "cash",
  seats: "6",
  difficulty: "mixed",
  stackBb: "100",
  autoRebuy: true,
  startingChips: "1500",
  levels: "10/20, 15/30, 25/50, 50/100, 75/150, 100/200, 150/300, 200/400, 300/600, 400/800",
  handsPerLevel: "10",
  leak: "none",
  payouts: "",
};

export type BuildResult = { ok: true; config: Record<string, unknown> } | { ok: false; errors: Record<string, string> };

export function parseLevels(s: string): { small_blind: number; big_blind: number; ante: number }[] | null {
  const out = [];
  for (const part of s
    .split(",")
    .map((p) => p.trim())
    .filter(Boolean)) {
    const nums = part.split("/").map((x) => parseScaled(x.trim(), 0));
    if (nums.length < 2 || nums.length > 3 || nums.some((n) => n === null)) return null;
    const [small_blind, big_blind, ante = 0] = nums as number[];
    out.push({ small_blind, big_blind, ante });
  }
  return out.length ? out : null;
}

const ORDER: Difficulty[] = ["easy", "medium", "hard"];

export function buildTableConfig(form: PlayForm, game: GameConfig, seed: number): BuildResult {
  const errors: Record<string, string> = {};
  const seats = parseScaled(form.seats, 0);
  if (seats === null || seats < 2 || seats > 6) errors.seats = "2–6";
  const n = seats ?? 2;
  const opponents = Array.from({ length: n - 1 }, (_, i) =>
    form.difficulty === "mixed" ? ORDER[i % 3] : form.difficulty,
  );
  let format: Record<string, unknown> = {};
  if (form.format === "sit_and_go") {
    const chips = parseScaled(form.startingChips, 0);
    if (chips === null || chips === 0) errors.startingChips = "Whole chips, e.g. 1500";
    const levels = parseLevels(form.levels);
    if (!levels) errors.levels = "e.g. 10/20, 15/30, 25/50/5 (small/big/ante)";
    const hands = parseScaled(form.handsPerLevel, 0);
    if (hands === null || hands === 0) errors.handsPerLevel = "A whole number of hands";
    const payouts = form.payouts.trim() ? form.payouts.split(/[/,]/).map((x) => parseScaled(x.trim(), 0)) : [];
    if (payouts.some((p) => p === null || p === 0)) errors.payouts = "Whole percentages, e.g. 65/35";
    format = { type: "sit_and_go", starting_chips: chips, levels, hands_per_level: hands, payouts };
  } else {
    const stack = bbToMbb(form.stackBb);
    if (stack === null || stack === 0) errors.stackBb = "Stack in bb (up to 3 decimals)";
    const g = { ...game, players: n, stack: { effective_stack: stack } };
    format =
      form.format === "cash" ? { type: "cash", game: g, auto_rebuy: form.autoRebuy } : { type: "fast_fold", game: g };
  }
  if (Object.keys(errors).length) return { ok: false, errors };
  const leaks = form.leak === "none" ? [] : opponents.map(() => form.leak);
  return { ok: true, config: { format, opponents, seed, leaks } };
}

/** Formats an amount in the table's units. */
export function amount(x: number, units: "bb" | "chips"): string {
  return units === "chips" ? x.toLocaleString() : `${formatBb(x)}bb`;
}

/** Converts a typed amount ("12.5" bb, or "450" chips) to table units, or null if inexact. */
export function parseAmount(s: string, units: "bb" | "chips"): number | null {
  return units === "chips" ? parseScaled(s, 0) : bbToMbb(s);
}

export const pct = (r: Ratio) => (r.opportunities ? `${((r.made / r.opportunities) * 100).toFixed(0)}%` : "—");

/** What the solver says about one of your decisions (engine play_engine::review::DecisionReview). */
export type DecisionReview = {
  decision: number;
  before: string[];
  actions: string[];
  strategy: number[] | null;
  action_evs_bb: number[] | null;
  taken: number;
  best: number | null;
  ev_loss_bb: number | null;
};

/** One street of a solver review of a hand. */
export type ReviewStreet = {
  street: string;
  decisions: number[];
  solvable: boolean;
  note: string | null;
  status: "pending" | "solving" | "done" | "failed" | "skipped";
  job_id?: string;
  job_status?: string;
  progress?: { iterations?: number; exploitability_pct_pot?: number };
  notes?: string[];
  error?: string;
  review?: { decisions: DecisionReview[]; followed: boolean; notes: string[]; next_ranges: string[] | null };
  /** The street's real line (solve units), your seat in the solve and your hand. */
  line?: { player: number; kind: string; to_mbb: number; all_in: boolean; decision: number | null }[];
  hero_player?: number;
  hero_combo?: string;
  hero_class?: string;
};

export type HandSolverReview = {
  id: string;
  status: "queued" | "running" | "done" | "failed" | "interrupted";
  streets: ReviewStreet[];
  error_message: string | null;
};

/**
 * A stored hand record with the fields that newer engines add filled in, so hands recorded by an
 * older version still display (the engine defaults them the same way when it reads them).
 */
export function withRecordDefaults(h: HandRecord): HandRecord {
  return {
    ...h,
    summary: h.summary ?? [],
    revealed: h.revealed ?? false,
    hero_decisions: (h.hero_decisions ?? []).map((d) => ({ ...d, feedback: d.feedback ?? null, spot: d.spot ?? null })),
  };
}

/** One bot action and what it did to that bot's tracked range (engine `RangeStep`). */
export type RangeStep = {
  street: string;
  action: string;
  source: "preflop_model" | "preflop_rules" | "postflop_rules";
  kept: number;
  combos_after: number;
};

export type OpponentRange = {
  player: number;
  name: string;
  position: string;
  combos: number;
  share_of_all: number;
  classes: ClassStrategy[];
  steps: RangeStep[];
  categories: { category: string; share: number; hero_equity: number }[];
  hero_equity: number | null;
  /** Saved copies only (hand review): the cards this bot really held, and how much of that exact hand its range kept. */
  held?: { cards: string[]; weight: number } | null;
  /** Saved copies only: the class grid packed as [row × 13 + col, weighted combos, share of the class]. */
  grid?: [number, number, number][];
};

const RANKS = "AKQJT98765432";

/** A 13×13 cell's hand class label: pairs on the diagonal, suited above, offsuit below. */
export function classLabel(row: number, col: number): string {
  if (row === col) return RANKS[row] + RANKS[row];
  return row < col ? `${RANKS[row]}${RANKS[col]}s` : `${RANKS[col]}${RANKS[row]}o`;
}

/** An opponent's range by class: as sent live, or unpacked from a saved copy's grid. */
export function rangeClasses(o: OpponentRange): ClassStrategy[] {
  if (o.classes.length > 0 || !o.grid) return o.classes;
  return o.grid.map(([cell, weight, share]) => {
    const row = Math.floor(cell / 13);
    const col = cell % 13;
    return { label: classLabel(row, col), row, col, weight, frequencies: [share] };
  });
}

/** Your equity at a decision, and what it is made of (engine `EquityBreakdown`). */
export type EquityBreakdown = {
  street: string;
  hero_cards: string[];
  board: string[];
  hero_hand: string | null;
  equity: number | null;
  method: "exact" | "monte_carlo";
  samples: number | null;
  required: number | null;
  opponents: OpponentRange[];
  hero_range: string | null;
};

export const HAND_CATEGORY: Record<string, string> = {
  high_card: "No pair",
  one_pair: "One pair",
  two_pair: "Two pair",
  three_of_a_kind: "Three of a kind",
  straight: "Straight",
  flush: "Flush",
  full_house: "Full house",
  four_of_a_kind: "Four of a kind",
  straight_flush: "Straight flush",
  royal_flush: "Royal flush",
};

/**
 * Play against bots: request/response types and the table-config builder. Exact unit
 * conversion only; the engine validates every rule (seats, blinds, stacks).
 */
import type { LineStep, SolveSpecIn } from "./solve";
import type { GameConfig } from "./types";
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
  caveats: string[];
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
    format = { type: "sit_and_go", starting_chips: chips, levels, hands_per_level: hands };
  } else {
    const stack = bbToMbb(form.stackBb);
    if (stack === null || stack === 0) errors.stackBb = "Stack in bb (up to 3 decimals)";
    const g = { ...game, players: n, stack: { effective_stack: stack } };
    format =
      form.format === "cash" ? { type: "cash", game: g, auto_rebuy: form.autoRebuy } : { type: "fast_fold", game: g };
  }
  if (Object.keys(errors).length) return { ok: false, errors };
  return { ok: true, config: { format, opponents, seed } };
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

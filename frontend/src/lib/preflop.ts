/**
 * Builds the backend's PreflopSpec from form input. Exact unit conversion only; every semantic
 * check (sizes against the stack, rake, player count) happens in the engine.
 */
import type { GameConfig } from "./types";
import { bbToMbb, parseScaled } from "./units";

export interface PreflopForm {
  /** "2" (heads-up: small blind vs big blind) or "3"–"6". */
  players: string;
  stackBb: string;
  /** Open-raise sizes, raise-to in bb, e.g. "2.5" or "2, 2.5". */
  opens: string;
  /** Re-raise sizes as multiples of the bet faced, e.g. "3" or "2.5, 3.5". */
  reraises: string;
  maxRaises: string;
  allInThresholdPct: string;
  addAllIn: boolean;
  allowLimp: boolean;
  realization: "positional" | "full";
  iterations: string;
  maxMinutes: string;
}

export const DEFAULT_PREFLOP_FORM: PreflopForm = {
  players: "6",
  stackBb: "100",
  opens: "2.5",
  reraises: "3",
  maxRaises: "2",
  allInThresholdPct: "85",
  addAllIn: false,
  allowLimp: false,
  realization: "positional",
  iterations: "300",
  maxMinutes: "20",
};

/** Heads-up push/fold: the one preflop tree RiverLab has validated against an exact solution. */
export function pushFoldForm(stackBb: string): PreflopForm {
  return {
    ...DEFAULT_PREFLOP_FORM,
    players: "2",
    stackBb,
    opens: "",
    reraises: "",
    maxRaises: "1",
    addAllIn: true,
    iterations: "1000",
  };
}

export type PreflopSpecResult =
  { ok: true; spec: Record<string, unknown> } | { ok: false; errors: Record<string, string> };

function list(s: string, scale: number): number[] | null {
  const out: number[] = [];
  for (const p of s.split(/[\s,]+/).filter(Boolean)) {
    const v = parseScaled(p.replace(/x$/i, ""), scale);
    if (v === null || v === 0) return null;
    out.push(v);
  }
  return out;
}

export function buildPreflopSpec(form: PreflopForm, game: GameConfig): PreflopSpecResult {
  const errors: Record<string, string> = {};
  const players = parseScaled(form.players, 0);
  if (players === null || players < 2 || players > 6) errors.players = "2–6";
  const stack = bbToMbb(form.stackBb);
  if (stack === null || stack === 0) errors.stackBb = "Enter the stack in bb (up to 3 decimals)";
  const opens = list(form.opens, 3);
  if (opens === null) errors.opens = "Raise-to sizes in bb, e.g. 2.5";
  // "3" -> 300 (% of the bet faced); two decimals allowed ("2.25x").
  const reraises = list(form.reraises, 2);
  if (reraises === null) errors.reraises = "Multiples of the bet faced, e.g. 3";
  const maxRaises = parseScaled(form.maxRaises, 0);
  if (maxRaises === null || maxRaises < 1 || maxRaises > 6) errors.maxRaises = "1–6";
  const threshold = parseScaled(form.allInThresholdPct, 0);
  if (threshold === null || threshold < 1 || threshold > 100) errors.allInThresholdPct = "1–100";
  const iterations = parseScaled(form.iterations, 0);
  if (iterations === null || iterations < 1) errors.iterations = "Enter a whole number of iterations";
  const minutes = form.maxMinutes.trim() === "" ? null : parseScaled(form.maxMinutes, 0);
  if (form.maxMinutes.trim() !== "" && (minutes === null || minutes === 0))
    errors.maxMinutes = "Whole minutes, or blank for no limit";
  if (Object.keys(errors).length) return { ok: false, errors };

  const headsUp = players === 2;
  return {
    ok: true,
    spec: {
      // The heads-up flag overrides the player count; keep the game's count valid (3–6) otherwise.
      game: { ...game, players: headsUp ? game.players : players, stack: { effective_stack: stack } },
      heads_up: headsUp,
      sizing: {
        open_raise_to_mbb: opens,
        reraise_pct_of_bet: reraises,
        max_raises: maxRaises,
        add_all_in: form.addAllIn,
        all_in_threshold_pct_stack: threshold,
        allow_limp: form.allowLimp,
      },
      realization: form.realization,
      max_iterations: iterations,
      max_seconds: minutes === null ? null : minutes * 60,
      check_interval: Math.max(1, Math.min(50, Math.floor((iterations as number) / 10))),
      threads: 1,
    },
  };
}

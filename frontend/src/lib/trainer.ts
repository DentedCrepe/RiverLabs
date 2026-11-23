/** Push/fold trainer types (engine play_engine::push_fold) and the session tally. */
import type { ClassStrategy } from "./types";

export type Seat = "sb" | "bb";
export type TableState = "missing" | "loading" | "building" | "ready" | "error";

export type TrainerStatus = {
  state: TableState;
  progress: number | null;
  message: string | null;
  table_present: boolean;
};

export type Drill = { stack_mbb: number; seat: Seat; cards: string[]; class: string };

export type Grade = {
  drill: Drill;
  choice: "fold" | "play";
  actions: string[];
  frequencies: [number, number];
  evs_bb: [number, number];
  ev_loss_bb: number;
  verdict: "correct" | "close" | "mistake";
  exploitability_bb: number;
};

export type Chart = {
  stack_mbb: number;
  sb: ClassStrategy[];
  bb: ClassStrategy[];
  sb_jam_share: number;
  bb_call_share: number;
  sb_ev_bb: number;
  exploitability_bb: number;
  iterations: number;
};

export type Tally = { hands: number; correct: number; close: number; mistakes: number; evLost: number };
export const EMPTY_TALLY: Tally = { hands: 0, correct: 0, close: 0, mistakes: 0, evLost: 0 };

export function addToTally(t: Tally, g: Grade): Tally {
  return {
    hands: t.hands + 1,
    correct: t.correct + (g.verdict === "correct" ? 1 : 0),
    close: t.close + (g.verdict === "close" ? 1 : 0),
    mistakes: t.mistakes + (g.verdict === "mistake" ? 1 : 0),
    evLost: t.evLost + g.ev_loss_bb,
  };
}

/** "+0.31bb" / "−0.5bb" with up to 3 decimals (display only; values come from the engine). */
export function signedBb(x: number, digits = 3): string {
  const v = Number(Math.abs(x).toFixed(digits));
  return `${x < 0 && v !== 0 ? "−" : "+"}${v}bb`;
}

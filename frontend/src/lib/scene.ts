/**
 * Academy question scenes (validated at sync; docs/academy.md) shown on the Play table. Only
 * layout here: who sits where, what is in front of them. No poker arithmetic.
 */
import type { SeatView, TableView } from "./play";

export type Scene = {
  players: number;
  /** Positions in preflop order, which is clockwise around the table. */
  seats: string[];
  hero: string;
  hero_cards: string[];
  board: string[];
  pot_mbb: number;
  bets_mbb: Record<string, number>;
  stacks_mbb: Record<string, number>;
  folded: string[];
  actions: Record<string, string>;
  /** Opponents' cards shown face up (e.g. at showdown). */
  shown?: Record<string, string[]>;
};

/** The scene as a table view: you at the bottom, the others clockwise. */
export function sceneToTable(s: Scene): TableView {
  const start = s.seats.indexOf(s.hero);
  const order = s.seats.map((_, i) => s.seats[(start + i) % s.seats.length]);
  // Heads-up the small blind is the button.
  const button = s.seats.includes("BTN") ? "BTN" : "SB";
  const seats: SeatView[] = order.map((pos) => {
    const folded = s.folded.includes(pos);
    return {
      name: pos === s.hero ? "You" : pos,
      is_hero: pos === s.hero,
      position: pos,
      is_button: pos === button,
      stack: s.stacks_mbb[pos] ?? 0,
      street_commit: s.bets_mbb[pos] ?? 0,
      folded,
      all_in: false,
      busted: false,
      finish: null,
      cards: pos === s.hero && s.hero_cards.length ? s.hero_cards : (s.shown?.[pos] ?? null),
      in_hand: !folded,
      last_action: s.actions[pos] ?? (folded ? "folded" : null),
      hidden_cards: null,
    };
  });
  return {
    status: "awaiting_hero",
    format: "cash",
    units: "bb",
    big_blind: 1000,
    small_blind: 500,
    ante: 0,
    level: null,
    hand_no: 0,
    street: null,
    board: s.board,
    // Like the Play table: the pot including this street's bets.
    pot: s.pot_mbb + Object.values(s.bets_mbb).reduce((a, b) => a + b, 0),
    seats,
    to_act: 0,
    legal: null,
    log: [],
    last_hand_net: null,
    hero_total_net: 0,
    notes: [],
    reveal: false,
    feedback: [],
    summary: [],
    hero_equity: null,
    required_equity: null,
  };
}

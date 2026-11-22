export type SuitCode = "s" | "h" | "d" | "c";

export const SUIT_SYMBOL: Record<SuitCode, string> = { s: "♠", h: "♥", d: "♦", c: "♣" };
export const SUIT_NAME: Record<SuitCode, string> = { s: "spades", h: "hearts", d: "diamonds", c: "clubs" };
const RANK_NAME: Record<string, string> = {
  A: "Ace",
  K: "King",
  Q: "Queen",
  J: "Jack",
  T: "Ten",
  "9": "Nine",
  "8": "Eight",
  "7": "Seven",
  "6": "Six",
  "5": "Five",
  "4": "Four",
  "3": "Three",
  "2": "Two",
};

/** Splits "AsKd" into ["As", "Kd"] for display. Validation is the engine's job. */
export function splitCards(s: string): string[] {
  const clean = s.replace(/[\s,]/g, "");
  const out: string[] = [];
  for (let i = 0; i + 1 < clean.length; i += 2) out.push(clean.slice(i, i + 2));
  return out;
}

export function cardName(card: string): string {
  const rank = RANK_NAME[card[0]?.toUpperCase() ?? ""] ?? card[0];
  const suit = SUIT_NAME[(card[1]?.toLowerCase() ?? "") as SuitCode] ?? card[1];
  return `${rank} of ${suit}`;
}

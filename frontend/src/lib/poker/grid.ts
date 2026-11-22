/**
 * Static 13×13 starting-hand grid layout (presentation data only; all range mathematics
 * happens in the engine). Row/column 0 = ace … 12 = deuce. Pairs on the diagonal, suited
 * hands above it, offsuit below — matching the engine's HandClass::grid_position.
 */

export const GRID_RANKS = ["A", "K", "Q", "J", "T", "9", "8", "7", "6", "5", "4", "3", "2"] as const;

export type HandKind = "pair" | "suited" | "offsuit";

export interface GridCell {
  label: string;
  row: number;
  col: number;
  kind: HandKind;
}

export function cellAt(row: number, col: number): GridCell {
  const [hi, lo] = row <= col ? [row, col] : [col, row];
  const kind: HandKind = row === col ? "pair" : row < col ? "suited" : "offsuit";
  const label = `${GRID_RANKS[hi]}${GRID_RANKS[lo]}${kind === "pair" ? "" : kind === "suited" ? "s" : "o"}`;
  return { label, row, col, kind };
}

export const GRID: readonly GridCell[] = Array.from({ length: 169 }, (_, i) => cellAt(Math.floor(i / 13), i % 13));

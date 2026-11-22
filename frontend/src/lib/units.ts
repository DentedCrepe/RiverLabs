/**
 * Exact conversions between user-entered decimals and the engine's integer units.
 * The engine works in milli-big-blinds (1bb = 1000 mbb) and basis points (500 = 5%).
 * Inputs with more precision than the unit allows are rejected, never rounded.
 */

const DECIMAL = /^\s*(\d+)(?:\.(\d*))?\s*$/;

/** Parses a non-negative decimal string into an integer number of 10^-scale units. */
export function parseScaled(input: string, scale: number): number | null {
  const m = DECIMAL.exec(input);
  if (!m) return null;
  const [, whole, frac = ""] = m;
  if (frac.length > scale) return null;
  const value = Number(whole) * 10 ** scale + Number(frac.padEnd(scale, "0") || "0");
  return Number.isSafeInteger(value) ? value : null;
}

export const bbToMbb = (bb: string): number | null => parseScaled(bb, 3);
export const percentToBps = (pct: string): number | null => parseScaled(pct, 2);

/** Formats mbb as an exact bb string ("2.5", "100", "0.001"). */
export function formatBb(mbb: number): string {
  const whole = Math.trunc(mbb / 1000);
  const frac = mbb % 1000;
  return frac === 0 ? String(whole) : `${whole}.${String(frac).padStart(3, "0").replace(/0+$/, "")}`;
}

export function formatBps(bps: number): string {
  const whole = Math.trunc(bps / 100);
  const frac = bps % 100;
  return frac === 0 ? String(whole) : `${whole}.${String(frac).padStart(2, "0").replace(/0+$/, "")}`;
}

export function formatPercent(fraction: number, digits = 2): string {
  return `${(fraction * 100).toFixed(digits)}%`;
}

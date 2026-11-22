/**
 * Form state for the game configuration editor and its exact conversion to the engine
 * document. Values are strings while editing; conversion rejects inexact input.
 */
import type { GameConfig, RakeAmount } from "@/lib/types";
import { bbToMbb, formatBb, formatBps, parseScaled, percentToBps } from "@/lib/units";

export type AmountUnit = "big_blinds" | "currency";

export interface AmountForm {
  enabled: boolean;
  unit: AmountUnit;
  value: string; // bb (decimal) or currency minor units (integer)
}

export interface ConfigForm {
  players: string;
  smallBlindBb: string;
  anteBb: string;
  stackBb: string;
  rakePercent: string;
  cap: AmountForm;
  minPot: AmountForm;
  roundingUnit: AmountUnit;
  roundingValue: string;
  roundingMode: "down" | "nearest" | "up";
  noFlopNoDrop: boolean;
  allocation: "proportional" | "main_pot_first";
  stakeEnabled: boolean;
  stakeCurrency: string;
  stakeBigBlindMinor: string;
}

function amountToForm(a: RakeAmount | null): AmountForm {
  if (!a) return { enabled: false, unit: "big_blinds", value: "" };
  return a.unit === "big_blinds"
    ? { enabled: true, unit: "big_blinds", value: formatBb(a.mbb) }
    : { enabled: true, unit: "currency", value: String(a.minor_units) };
}

export function configToForm(c: GameConfig): ConfigForm {
  const rounding = amountToForm(c.rake.rounding.increment);
  return {
    players: String(c.players),
    smallBlindBb: formatBb(c.small_blind),
    anteBb: formatBb(c.ante),
    stackBb: formatBb(c.stack.effective_stack),
    rakePercent: formatBps(c.rake.percentage_bps),
    cap: amountToForm(c.rake.cap),
    minPot: amountToForm(c.rake.min_pot),
    roundingUnit: rounding.unit,
    roundingValue: rounding.value,
    roundingMode: c.rake.rounding.mode,
    noFlopNoDrop: c.rake.no_flop_no_drop,
    allocation: c.rake.allocation,
    stakeEnabled: c.stake !== null,
    stakeCurrency: c.stake?.currency ?? "USD",
    stakeBigBlindMinor: c.stake ? String(c.stake.big_blind_minor_units) : "100",
  };
}

export type ConversionResult = { ok: true; config: GameConfig } | { ok: false; errors: Record<string, string> };

function toAmount(unit: AmountUnit, value: string): RakeAmount | null {
  if (unit === "big_blinds") {
    const mbb = bbToMbb(value);
    return mbb === null ? null : { unit: "big_blinds", mbb };
  }
  const minor = parseScaled(value, 0);
  return minor === null ? null : { unit: "currency", minor_units: minor };
}

/** Converts the form to an engine document. Only shape/precision is checked here; the
 * engine performs all semantic validation. */
export function formToConfig(f: ConfigForm, schemaVersion = 1): ConversionResult {
  const errors: Record<string, string> = {};
  const need = <T>(key: string, v: T | null, msg: string): T => {
    if (v === null) errors[key] = msg;
    return v as T;
  };
  const bbMsg = "Enter a number of big blinds with at most 3 decimals";
  const players = need("players", parseScaled(f.players, 0), "Enter a whole number");
  const small_blind = need("smallBlindBb", bbToMbb(f.smallBlindBb), bbMsg);
  const ante = need("anteBb", bbToMbb(f.anteBb), bbMsg);
  const effective_stack = need("stackBb", bbToMbb(f.stackBb), bbMsg);
  const percentage_bps = need("rakePercent", percentToBps(f.rakePercent), "Enter a percentage with at most 2 decimals");
  const amountMsg = (u: AmountUnit) => (u === "big_blinds" ? bbMsg : "Enter whole currency minor units (e.g. cents)");
  const cap = f.cap.enabled ? need("cap", toAmount(f.cap.unit, f.cap.value), amountMsg(f.cap.unit)) : null;
  const min_pot = f.minPot.enabled
    ? need("minPot", toAmount(f.minPot.unit, f.minPot.value), amountMsg(f.minPot.unit))
    : null;
  const increment = need("rounding", toAmount(f.roundingUnit, f.roundingValue), amountMsg(f.roundingUnit));
  const bbMinor = f.stakeEnabled ? need("stake", parseScaled(f.stakeBigBlindMinor, 0), "Enter whole minor units") : 0;
  if (Object.keys(errors).length > 0) return { ok: false, errors };
  return {
    ok: true,
    config: {
      schema_version: schemaVersion,
      players,
      small_blind,
      ante,
      stack: { effective_stack },
      rake: {
        percentage_bps,
        cap,
        min_pot,
        rounding: { increment, mode: f.roundingMode },
        no_flop_no_drop: f.noFlopNoDrop,
        allocation: f.allocation,
      },
      stake: f.stakeEnabled ? { currency: f.stakeCurrency.trim(), big_blind_minor_units: bbMinor } : null,
    },
  };
}

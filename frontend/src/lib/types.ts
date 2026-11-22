/** Types mirroring the backend API (backend/src/riverlab_api/schemas). */

export type Readiness = { status: "ok" | "degraded"; database: "ok" | "unavailable"; engine: string };

export type CapabilityStatus = "available" | "interface_only" | "planned";
export type Capability = { key: string; name: string; status: CapabilityStatus; detail: string };
export type Meta = { api_version: string; engine_version: string; capabilities: Capability[] };

export type RakeAmount = { unit: "big_blinds"; mbb: number } | { unit: "currency"; minor_units: number };
export type RakeConfig = {
  percentage_bps: number;
  cap: RakeAmount | null;
  min_pot: RakeAmount | null;
  rounding: { increment: RakeAmount; mode: "down" | "nearest" | "up" };
  no_flop_no_drop: boolean;
  allocation: "proportional" | "main_pot_first";
};
export type GameConfig = {
  schema_version: number;
  players: number;
  small_blind: number;
  ante: number;
  stack: { effective_stack: number };
  rake: RakeConfig;
  stake: { currency: string; big_blind_minor_units: number } | null;
};
export type SavedGameConfig = {
  id: string;
  name: string;
  builtin: boolean;
  stack_preset: string | null;
  config: GameConfig;
};
export type StackPresets = {
  presets: { label: string; big_blinds: number; mbb: number }[];
  default: string;
  min_mbb: number;
  max_mbb: number;
};
export type RakeResult = {
  rake: number;
  capped: boolean;
  reason: "no_flop_no_drop" | "below_minimum_pot" | "zero_rake" | "percentage" | "capped";
};

export type HandEvaluation = {
  value: number;
  category: string;
  category_name: string;
  description: string;
  best_five: string[];
};
export type CompareResult = { results: HandEvaluation[]; winners: number[] };

export type MatrixCell = {
  label: string;
  row: number;
  col: number;
  kind: "pair" | "suited" | "offsuit";
  combos: number;
  live_combos: number;
  weight_sum: number;
};
export type RangeAnalysis = {
  notation: string;
  combos: number;
  live_combos: number;
  fraction_of_all_hands: number;
  blocked_fraction: number;
  matrix: MatrixCell[];
};

export type EquityResult = {
  method: "exact" | "monte_carlo";
  equity: number[];
  win: number[];
  tie: number[];
  samples: number;
  std_error: number[] | null;
  seed: number | null;
};

export type SolverCapabilities = {
  status: string;
  message: string;
  available_backends: string[];
  planned_algorithms: string[];
};

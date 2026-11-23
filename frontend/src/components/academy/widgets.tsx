"use client";

/**
 * Interactive lesson components. Every number comes from the engine through the API
 * (`/theory/*`, `/ranges/analyze`, `/equity`); this file only converts units and draws.
 * Each widget states the model its numbers belong to.
 */

import { type ReactNode, useEffect, useState } from "react";
import { ApiError, api } from "@/lib/api";
import { bbToMbb, formatBb, parseScaled } from "@/lib/units";
import { Field, TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";

const pct = (x: number | null | undefined, d = 2) => (x === null || x === undefined ? "—" : `${(x * 100).toFixed(d)}%`);

/** Posts `body` to `path` whenever it changes (debounced); `body = null` means invalid input. */
function useCalc<T>(path: string, body: unknown | null): { data: T | null; error: string | null } {
  const key = body === null ? null : JSON.stringify(body);
  const [state, setState] = useState<{ data: T | null; error: string | null }>({ data: null, error: null });
  useEffect(() => {
    if (key === null) return;
    let cancelled = false;
    const t = setTimeout(() => {
      api
        .post<T>(path, JSON.parse(key))
        .then((data) => !cancelled && setState({ data, error: null }))
        .catch(
          (e: unknown) => !cancelled && setState({ data: null, error: e instanceof ApiError ? e.message : String(e) }),
        );
    }, 250);
    return () => {
      cancelled = true;
      clearTimeout(t);
    };
  }, [path, key]);
  return key === null ? { data: null, error: "Enter valid numbers (bb up to 3 decimals)" } : state;
}

function Frame({ title, assumptions, children }: { title: string; assumptions: string; children: ReactNode }) {
  return (
    <figure className="my-4 rounded-md border border-line bg-surface-2/40 p-3" aria-label={title}>
      <figcaption className="mb-2 text-xs font-semibold tracking-wide text-muted uppercase">{title}</figcaption>
      {children}
      <p className="mt-2 text-[11px] text-subtle">Model: {assumptions}</p>
    </figure>
  );
}

function Num({
  label,
  value,
  onChange,
  hint,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  hint?: string;
}) {
  return (
    <Field label={label} hint={hint}>
      {(id, d) => (
        <TextInput
          id={id}
          aria-describedby={d}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="w-28"
        />
      )}
    </Field>
  );
}

function Out({ label, value, strong }: { label: string; value: string; strong?: boolean }) {
  return (
    <div>
      <dt className="text-xs text-muted">{label}</dt>
      <dd
        className={strong ? "font-mono text-base font-semibold text-fg tabular" : "font-mono text-sm text-fg tabular"}
      >
        {value}
      </dd>
    </div>
  );
}

type PotOddsResult = { required_equity: number; odds_ratio: number; final_pot_mbb: number };

export function PotOddsWidget({ pot_bb = "10", bet_bb = "5" }: { pot_bb?: string; bet_bb?: string }) {
  const [pot, setPot] = useState(String(pot_bb));
  const [bet, setBet] = useState(String(bet_bb));
  const [eq, setEq] = useState("30");
  const p = bbToMbb(pot);
  const b = bbToMbb(bet);
  const e = parseScaled(eq, 2);
  const valid = p !== null && b !== null && b > 0;
  const odds = useCalc<PotOddsResult>(
    "/api/v1/theory/pot_odds",
    valid ? { params: { pot_mbb: p + b, to_call_mbb: b } } : null,
  );
  const ev = useCalc<number>(
    "/api/v1/theory/ev_call",
    valid && e !== null && e <= 10000 ? { params: { equity: e / 10000, pot_mbb: p + b, to_call_mbb: b } } : null,
  );
  return (
    <Frame title="Pot odds calculator" assumptions="no more betting after the call (river or all-in), no rake.">
      <div className="flex flex-wrap gap-3">
        <Num label="Pot before the bet (bb)" value={pot} onChange={setPot} />
        <Num label="Bet you face (bb)" value={bet} onChange={setBet} />
        <Num label="Your equity (%)" value={eq} onChange={setEq} hint="Against their betting range" />
      </div>
      {odds.error ? (
        <ErrorState message={odds.error} />
      ) : (
        odds.data && (
          <dl className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Out label="Break-even equity" value={pct(odds.data.required_equity)} strong />
            <Out label="Odds" value={`${odds.data.odds_ratio.toFixed(2)} : 1`} />
            <Out label="Final pot" value={`${formatBb(odds.data.final_pot_mbb)}bb`} />
            <Out
              label="EV of calling vs folding"
              value={ev.data === null ? "—" : `${ev.data >= 0 ? "+" : "−"}${(Math.abs(ev.data) / 1000).toFixed(3)}bb`}
            />
          </dl>
        )
      )}
    </Frame>
  );
}

type OutsResult = { next_card: number; two_cards: number | null; rule_of_two: number; rule_of_four: number };

export function OutsWidget({ outs = 9, unseen = 47 }: { outs?: number; unseen?: number }) {
  const [o, setO] = useState(String(outs));
  const [u, setU] = useState(String(unseen));
  const on = parseScaled(o, 0);
  const un = parseScaled(u, 0);
  const r = useCalc<OutsResult>(
    "/api/v1/theory/outs",
    on !== null && un !== null ? { params: { outs: on, unseen: un } } : null,
  );
  return (
    <Frame
      title="Outs: exact vs rule of 2 and 4"
      assumptions="every out wins and every other card loses; unseen cards are equally likely."
    >
      <div className="flex flex-wrap gap-3">
        <Num label="Outs" value={o} onChange={setO} />
        <Num label="Unseen cards" value={u} onChange={setU} hint="47 on the flop, 46 on the turn" />
      </div>
      {r.error ? (
        <ErrorState message={r.error} />
      ) : (
        r.data && (
          <dl className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Out label="Next card (exact)" value={pct(r.data.next_card)} strong />
            <Out label="Rule of 2" value={pct(r.data.rule_of_two, 0)} />
            <Out label="Two cards (exact)" value={pct(r.data.two_cards)} strong />
            <Out label="Rule of 4" value={pct(r.data.rule_of_four, 0)} />
          </dl>
        )
      )}
    </Frame>
  );
}

type RangeResult = {
  combos: number;
  live_combos: number;
  matrix: { label: string; combos: number; live_combos: number; weight_sum: number }[];
};

export function CombosWidget({ range = "AK,KK", dead = "" }: { range?: string; dead?: string }) {
  const [notation, setNotation] = useState(String(range));
  const [cards, setCards] = useState(String(dead));
  const r = useCalc<RangeResult>(
    "/api/v1/ranges/analyze",
    notation.trim() ? { notation, dead: cards.replace(/\s/g, "") } : null,
  );
  const rows = r.data?.matrix.filter((c) => c.weight_sum > 0) ?? [];
  return (
    <Frame
      title="Combination counter"
      assumptions="exact counting; seen cards (board, your hand) cannot be in the range."
    >
      <div className="flex flex-wrap gap-3">
        <Field label="Range">
          {(id) => (
            <TextInput id={id} value={notation} onChange={(e) => setNotation(e.target.value)} className="w-56" />
          )}
        </Field>
        <Field label="Seen cards" hint="e.g. Kh7d2c">
          {(id) => <TextInput id={id} value={cards} onChange={(e) => setCards(e.target.value)} className="w-32" />}
        </Field>
      </div>
      {r.error ? (
        <ErrorState message={r.error} />
      ) : (
        r.data && (
          <>
            <dl className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
              <Out label="Combos" value={String(r.data.combos)} />
              <Out label="Still possible" value={String(r.data.live_combos)} strong />
            </dl>
            <ul className="mt-2 flex flex-wrap gap-2 font-mono text-xs" aria-label="Combos per hand">
              {rows.map((c) => (
                <li key={c.label} className="rounded border border-line px-1.5 py-0.5">
                  {c.label}{" "}
                  <span className="text-muted">
                    {c.live_combos}/{c.combos}
                  </span>
                </li>
              ))}
            </ul>
          </>
        )
      )}
    </Frame>
  );
}

type BetMath = {
  mdf: number;
  alpha: number;
  caller_required_equity: number;
  bluff_share: number;
  bluffs_per_value_bet: number;
};

export function BetMathWidget({ pot_bb = "10", bet_bb = "10" }: { pot_bb?: string; bet_bb?: string }) {
  const [pot, setPot] = useState(String(pot_bb));
  const [bet, setBet] = useState(String(bet_bb));
  const p = bbToMbb(pot);
  const b = bbToMbb(bet);
  const valid = p !== null && b !== null && p > 0 && b > 0;
  const m = useCalc<BetMath>("/api/v1/theory/bet_math", valid ? { params: { pot_mbb: p, bet_mbb: b } } : null);
  return (
    <Frame
      title="One river bet: alpha, MDF and bluff ratio"
      assumptions="one bet on the river, call or fold only; polarised bettor (nuts or air) against bluff-catchers."
    >
      <div className="flex flex-wrap gap-3">
        <Num label="Pot before the bet (bb)" value={pot} onChange={setPot} />
        <Num label="Bet (bb)" value={bet} onChange={setBet} />
      </div>
      {m.error ? (
        <ErrorState message={m.error} />
      ) : (
        m.data &&
        valid && (
          <>
            <dl className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-5">
              <Out label="Alpha (bluff needs folds)" value={pct(m.data.alpha, 1)} strong />
              <Out label="MDF" value={pct(m.data.mdf, 1)} strong />
              <Out label="Caller needs" value={pct(m.data.caller_required_equity, 1)} />
              <Out label="Bluff share of bets" value={pct(m.data.bluff_share, 1)} />
              <Out label="Bluffs per value bet" value={m.data.bluffs_per_value_bet.toFixed(3)} />
            </dl>
            <BluffEvChart pot={p} bet={b} alpha={m.data.alpha} />
          </>
        )
      )}
    </Frame>
  );
}

/** EV of a pure bluff against fold frequency: a straight line from −bet (never folds) to +pot. */
function BluffEvChart({ pot, bet, alpha }: { pot: number; bet: number; alpha: number }) {
  const never = useCalc<number>("/api/v1/theory/ev_bluff", {
    params: { fold_frequency: 0, pot_mbb: pot, bet_mbb: bet },
  });
  const always = useCalc<number>("/api/v1/theory/ev_bluff", {
    params: { fold_frequency: 1, pot_mbb: pot, bet_mbb: bet },
  });
  if (never.data === null || always.data === null) return null;
  const lo = never.data;
  const hiV = always.data;
  const W = 320;
  const H = 120;
  const max = Math.max(Math.abs(lo), Math.abs(hiV));
  const y = (v: number) => H / 2 - (v / max) * (H / 2 - 8);
  return (
    <svg
      viewBox={`0 0 ${W} ${H + 18}`}
      className="mt-3 w-full max-w-sm"
      role="img"
      aria-label="EV of a pure bluff by fold frequency"
    >
      <line x1={0} x2={W} y1={H / 2} y2={H / 2} className="stroke-line" />
      <line x1={0} x2={W} y1={y(lo)} y2={y(hiV)} className="stroke-accent" strokeWidth={2} />
      <line x1={alpha * W} x2={alpha * W} y1={0} y2={H} className="stroke-warn" strokeDasharray="3 3" />
      <text x={alpha * W + 4} y={12} className="fill-warn text-[10px]">
        break-even {pct(alpha, 1)}
      </text>
      <text x={0} y={H + 14} className="fill-muted text-[10px]">
        folds 0%
      </text>
      <text x={W} y={H + 14} textAnchor="end" className="fill-muted text-[10px]">
        folds 100%
      </text>
    </svg>
  );
}

type RmStep = { iteration: number; current: number[]; average: number[]; regrets: number[]; exploitability: number };

export function RegretMatchingWidget({
  iterations = 300,
  start_regrets = [3, 0, 0],
}: {
  iterations?: number;
  start_regrets?: number[];
}) {
  const [n, setN] = useState(String(iterations));
  const [plus, setPlus] = useState(false);
  const it = parseScaled(n, 0);
  const r = useCalc<RmStep[]>(
    "/api/v1/theory/regret_matching",
    it !== null && it > 0 && it <= 10000 ? { params: { iterations: it, start_regrets, plus } } : null,
  );
  const steps = r.data ?? [];
  const last = steps.at(-1);
  const W = 320;
  const H = 110;
  const path = (f: (s: RmStep) => number) =>
    steps
      .map(
        (s, i) =>
          `${i ? "L" : "M"}${((i / Math.max(steps.length - 1, 1)) * W).toFixed(1)},${(H - f(s) * H).toFixed(1)}`,
      )
      .join(" ");
  const maxExp = Math.max(...steps.map((s) => s.exploitability), 1e-9);
  return (
    <Frame
      title="Regret matching on rock–paper–scissors"
      assumptions="a toy two-player zero-sum game (not poker); both players use the same rule from the same start."
    >
      <div className="flex flex-wrap items-end gap-3">
        <Num label="Iterations" value={n} onChange={setN} hint="1–10,000" />
        <label className="mb-2 inline-flex items-center gap-2 text-sm text-fg">
          <input type="checkbox" className="size-4" checked={plus} onChange={(e) => setPlus(e.target.checked)} />
          Regret matching+ (floor regrets at 0)
        </label>
      </div>
      {r.error ? (
        <ErrorState message={r.error} />
      ) : (
        last && (
          <>
            <svg
              viewBox={`0 0 ${W} ${H}`}
              className="mt-3 w-full max-w-md"
              role="img"
              aria-label="Probability of rock over iterations"
            >
              <line x1={0} x2={W} y1={H - H / 3} y2={H - H / 3} className="stroke-line" strokeDasharray="2 3" />
              <path d={path((s) => s.current[0])} fill="none" className="stroke-warn" strokeWidth={1} />
              <path d={path((s) => s.average[0])} fill="none" className="stroke-accent" strokeWidth={2} />
              <path
                d={path((s) => s.exploitability / maxExp)}
                fill="none"
                className="stroke-danger"
                strokeWidth={1}
                strokeDasharray="4 2"
              />
            </svg>
            <p className="text-[11px] text-muted">
              <span className="text-warn">Current</span> and <span className="text-accent">average</span> probability of
              rock (dotted line: 1/3); <span className="text-danger">exploitability</span> of the average, scaled.
            </p>
            <dl className="mt-2 grid grid-cols-2 gap-3 sm:grid-cols-4">
              <Out label="Average strategy" value={last.average.map((x) => x.toFixed(3)).join(" / ")} />
              <Out label="Current strategy" value={last.current.map((x) => x.toFixed(3)).join(" / ")} />
              <Out label="Exploitability of average" value={last.exploitability.toFixed(4)} strong />
              <Out label="Iterations" value={String(last.iteration)} />
            </dl>
          </>
        )
      )}
    </Frame>
  );
}

type EquityResult = { equity: number[]; method: string; samples: number };

export function EquityWidget({ hands = ["AsKs", "QhQd"], board = "" }: { hands?: string[]; board?: string }) {
  const [a, setA] = useState(String(hands[0] ?? ""));
  const [b, setB] = useState(String(hands[1] ?? ""));
  const [bd, setBd] = useState(String(board));
  const r = useCalc<EquityResult>(
    "/api/v1/equity",
    a && b ? { ranges: [a, b], board: bd.replace(/\s/g, ""), method: "exact" } : null,
  );
  return (
    <Frame title="Equity" assumptions="exact enumeration of every remaining board; no folding.">
      <div className="flex flex-wrap gap-3">
        <Field label="Hand or range 1">
          {(id) => <TextInput id={id} value={a} onChange={(e) => setA(e.target.value)} className="w-32" />}
        </Field>
        <Field label="Hand or range 2">
          {(id) => <TextInput id={id} value={b} onChange={(e) => setB(e.target.value)} className="w-32" />}
        </Field>
        <Field label="Board">
          {(id) => <TextInput id={id} value={bd} onChange={(e) => setBd(e.target.value)} className="w-32" />}
        </Field>
      </div>
      {r.error ? (
        <ErrorState message={r.error} />
      ) : (
        r.data && (
          <dl className="mt-3 grid grid-cols-2 gap-3">
            <Out label={a} value={pct(r.data.equity[0])} strong />
            <Out label={b} value={pct(r.data.equity[1])} strong />
          </dl>
        )
      )}
    </Frame>
  );
}

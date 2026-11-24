"use client";

import { RotateCw } from "lucide-react";
import { useState } from "react";
import { ApiError, api } from "@/lib/api";
import { type HandRecord, type SeatView, type TableView, describeAction } from "@/lib/play";
import { formatBb } from "@/lib/units";
import { useApi } from "@/lib/useApi";
import { CardRow } from "@/components/poker/CardText";
import { TableFelt } from "@/components/play/TableFelt";
import { SolverDecision } from "@/components/play/SolverReview";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, Select } from "@/components/ui/Field";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

type Source = {
  job_id: string;
  board: string[];
  starting_pot_mbb: number;
  effective_stack_mbb: number;
  exploitability_pct_pot: number | null;
};

export type PostflopDrill = {
  hero_player: number;
  hero_combo: string;
  hero_class: string;
  path: unknown[];
  history: { player: number | null; label: string }[];
  street: string;
  board: string[];
  pot_mbb: number;
  street_bets_mbb: [number, number];
  stacks_mbb: [number, number];
  actions: string[];
};

type Dealt = {
  job_id: string;
  drill: PostflopDrill;
  starting_pot_mbb: number;
  exploitability_pct_pot: number | null;
  /** Set when the solve came from a hand you played (a solver review). */
  hand?: { session_id: string; hand_no: number; street: string; record: HandRecord } | null;
};

const STREETS = ["preflop", "flop", "turn", "river"];
const BOARD_AT: Record<string, number> = { preflop: 0, flop: 3, turn: 4, river: 5 };

/** The real hand up to the street this solve starts on. */
export function HandSoFar({ hand }: { hand: NonNullable<Dealt["hand"]> }) {
  const h = hand.record;
  const units = h.tournament ? "chips" : "bb";
  const hero = h.seats.findIndex((s) => s.is_hero);
  const before = STREETS.slice(0, STREETS.indexOf(hand.street));
  return (
    <section
      aria-label="The hand so far"
      className="flex flex-col gap-1 rounded-md border border-line px-3 py-2 text-xs"
    >
      <p className="font-medium text-fg">
        From hand #{hand.hand_no} you played. Before the {hand.street}:
      </p>
      {before.map((st) => {
        const acts = h.actions.filter((a) => a.street === st);
        return (
          <p key={st} className="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-muted">
            <span className="text-fg capitalize">{st}</span>
            {BOARD_AT[st] > 0 && <CardRow cards={h.board.slice(0, BOARD_AT[st])} size="sm" />}
            <span>
              {acts
                .map(
                  (a) =>
                    `${a.seat === hero ? "You" : `${h.seats[a.seat].position} ${h.seats[a.seat].name}`} ${describeAction(a.action, units, a.all_in, a.seat === hero)}`,
                )
                .join(", ") || "no action"}
            </span>
          </p>
        );
      })}
      <p className="text-[11px] text-subtle">
        The drill deals both hands at random from the solve&apos;s ranges, so the cards and the action on the{" "}
        {hand.street} can differ from the real hand.
      </p>
    </section>
  );
}

type Grade = {
  actions: string[];
  strategy: number[];
  action_evs_bb: number[];
  taken: number;
  best: number;
  ev_loss_bb: number;
  ev_loss_pct_pot: number;
  verdict: "correct" | "close" | "mistake";
};

const SIDE = ["OOP", "IP"];
const VERDICT: Record<Grade["verdict"], { label: string; style: string }> = {
  correct: { label: "Correct", style: "bg-accent-dim text-accent-strong" },
  close: { label: "Close", style: "bg-warn-dim text-warn" },
  mistake: { label: "Mistake", style: "bg-danger-dim text-danger" },
};

/** The drill drawn on the Play table: you at the bottom, your opponent opposite. */
export function drillTable(d: PostflopDrill): TableView {
  const lastOnStreet = (p: number) => {
    const sinceCard = d.history.slice(d.history.map((h) => h.player).lastIndexOf(null) + 1);
    return [...sinceCard].reverse().find((h) => h.player === p)?.label ?? null;
  };
  const seat = (p: number): SeatView => ({
    name: p === d.hero_player ? "You" : "Opponent",
    is_hero: p === d.hero_player,
    position: SIDE[p],
    is_button: p === 1,
    stack: Math.round(d.stacks_mbb[p]),
    street_commit: Math.round(d.street_bets_mbb[p]),
    folded: false,
    all_in: false,
    busted: false,
    finish: null,
    cards: p === d.hero_player ? [d.hero_combo.slice(0, 2), d.hero_combo.slice(2, 4)] : null,
    in_hand: true,
    last_action: lastOnStreet(p),
    hidden_cards: null,
  });
  const other = 1 - d.hero_player;
  return {
    status: "awaiting_hero",
    format: "cash",
    units: "bb",
    big_blind: 1000,
    small_blind: 500,
    ante: 0,
    level: null,
    hand_no: 0,
    street: d.street,
    board: d.board,
    pot: Math.round(d.pot_mbb + d.street_bets_mbb[0] + d.street_bets_mbb[1]),
    seats: [seat(d.hero_player), seat(other)],
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

/** "Flop: OOP check, IP bet 3.3bb · Turn 7♥: OOP check" */
function historyText(d: PostflopDrill): string {
  const parts: string[] = [];
  let street = ["flop", "turn", "river"][
    Math.max(0, d.board.length - (d.history.filter((h) => h.player === null).length + 3))
  ];
  let current: string[] = [];
  const flush = () => {
    if (current.length) parts.push(`${street[0].toUpperCase()}${street.slice(1)}: ${current.join(", ")}`);
    current = [];
  };
  for (const h of d.history) {
    if (h.player === null) {
      flush();
      street = street === "flop" ? "turn" : "river";
      parts.push(`${street[0].toUpperCase()}${street.slice(1)} ${h.label}`);
    } else {
      current.push(`${SIDE[h.player]} ${h.label}`);
    }
  }
  flush();
  return parts.join(" · ");
}

export function PostflopTrainer() {
  const sources = useApi<Source[]>("/api/v1/trainer/postflop/sources");
  const [jobId, setJobId] = useState("");
  const [side, setSide] = useState<"any" | "0" | "1">("any");
  const [dealt, setDealt] = useState<Dealt | null>(null);
  const [grade, setGrade] = useState<Grade | null>(null);
  const [tally, setTally] = useState({ n: 0, correct: 0, lost: 0 });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function deal() {
    setBusy(true);
    setError(null);
    try {
      const d = await api.post<Dealt>("/api/v1/trainer/postflop/drill", {
        job_id: jobId || null,
        hero: side === "any" ? null : Number(side),
      });
      setDealt(d);
      setGrade(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function answer(choice: number) {
    if (!dealt) return;
    setBusy(true);
    setError(null);
    try {
      const g = await api.post<Grade>("/api/v1/trainer/postflop/answer", {
        job_id: dealt.job_id,
        drill: dealt.drill,
        choice,
      });
      setGrade(g);
      setTally((t) => ({
        n: t.n + 1,
        correct: t.correct + (g.verdict === "correct" ? 1 : 0),
        lost: t.lost + g.ev_loss_bb,
      }));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (sources.error) return <ErrorState message={sources.error.message} />;
  if (sources.loading && !sources.data) return <LoadingState label="Loading saved solves…" />;
  if (!sources.data?.length) {
    return (
      <EmptyState
        title="No saved solves yet"
        description="Postflop drills are dealt from your own saved solves. Solve a spot on the Solver page, or review a hand you played with the solver, then come back."
      />
    );
  }
  const d = dealt?.drill;
  return (
    <div className="grid gap-6">
      <Card>
        <CardHeader title="Postflop drill" subtitle="A decision from one of your saved solves, graded by EV" />
        <CardBody className="flex flex-col gap-4">
          <div className="grid gap-3 sm:max-w-xl sm:grid-cols-2">
            <Field label="Solve">
              {(id) => (
                <Select id={id} value={jobId} onChange={(e) => setJobId(e.target.value)}>
                  <option value="">Any saved solve</option>
                  {sources.data?.map((s) => (
                    <option key={s.job_id} value={s.job_id}>
                      {s.board.join(" ")} · pot {formatBb(s.starting_pot_mbb)}bb
                      {s.exploitability_pct_pot !== null
                        ? ` · ${s.exploitability_pct_pot.toFixed(2)}% exploitable`
                        : ""}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
            <Field label="Your position">
              {(id) => (
                <Select id={id} value={side} onChange={(e) => setSide(e.target.value as "any" | "0" | "1")}>
                  <option value="any">Either</option>
                  <option value="0">Out of position</option>
                  <option value="1">In position</option>
                </Select>
              )}
            </Field>
          </div>
          {d && dealt && (
            <div className="flex flex-col gap-3" aria-label="Current postflop drill">
              {dealt.hand && <HandSoFar hand={dealt.hand} />}
              <div className="w-full max-w-xl self-center">
                <TableFelt t={drillTable(d)} showAll={false} />
              </div>
              <p className="text-xs text-muted">
                You are {d.hero_player === 0 ? "out of position" : "in position"} with{" "}
                <CardRow cards={[d.hero_combo.slice(0, 2), d.hero_combo.slice(2, 4)]} size="sm" /> ({d.hero_class}).{" "}
                {d.history.length ? historyText(d) : "First to act on the street."}
              </p>
              {!grade ? (
                <div className="flex flex-wrap gap-2" role="group" aria-label="Your action">
                  {d.actions.map((a, i) => (
                    <Button key={a} onClick={() => answer(i)} disabled={busy}>
                      {a}
                    </Button>
                  ))}
                </div>
              ) : (
                <div className="flex flex-col gap-2 rounded-md border border-line bg-surface-2/50 px-3 py-2">
                  <p className="flex flex-wrap items-center gap-2 text-sm">
                    <span
                      className={cn(
                        "rounded px-1.5 py-px text-xs font-semibold uppercase",
                        VERDICT[grade.verdict].style,
                      )}
                    >
                      {VERDICT[grade.verdict].label}
                    </span>
                    <span className="text-fg">
                      {grade.ev_loss_bb > 0
                        ? `You lost ${grade.ev_loss_bb.toFixed(2)}bb (${grade.ev_loss_pct_pot.toFixed(1)}% of the starting pot).`
                        : "No EV lost."}
                    </span>
                  </p>
                  <SolverDecision
                    r={{
                      decision: 0,
                      before: [],
                      actions: grade.actions,
                      strategy: grade.strategy,
                      action_evs_bb: grade.action_evs_bb,
                      taken: grade.taken,
                      best: grade.best,
                      ev_loss_bb: grade.ev_loss_bb,
                    }}
                  />
                  <p className="text-[11px] text-subtle">
                    EVs against your opponent&apos;s range at this point, both players following the solve afterwards.
                    The solve is an approximate equilibrium for its bet sizes
                    {dealt.exploitability_pct_pot !== null &&
                      `, exploitable by ${dealt.exploitability_pct_pot.toFixed(2)}% of the pot`}
                    .
                  </p>
                </div>
              )}
            </div>
          )}
          <div className="flex flex-wrap items-center gap-3">
            <Button
              variant={d && !grade ? "secondary" : "primary"}
              onClick={deal}
              loading={busy && !d}
              icon={<RotateCw className="size-4" />}
            >
              {d ? "Next spot" : "Deal"}
            </Button>
            {tally.n > 0 && (
              <span className="font-mono text-xs text-muted tabular" aria-label="Postflop tally">
                {tally.n} answered · {Math.round((tally.correct / tally.n) * 100)}% correct · EV lost{" "}
                {(tally.lost / tally.n).toFixed(2)}bb per spot
              </span>
            )}
          </div>
          {error && <ErrorState message={error} />}
        </CardBody>
      </Card>
    </div>
  );
}

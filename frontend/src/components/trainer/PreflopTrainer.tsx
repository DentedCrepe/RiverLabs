"use client";

import { RotateCw } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { ApiError, api } from "@/lib/api";
import type { SeatView, TableView } from "@/lib/play";
import type { ClassStrategy } from "@/lib/types";
import { formatBb } from "@/lib/units";
import { useApi } from "@/lib/useApi";
import { CardRow } from "@/components/poker/CardText";
import { TableFelt } from "@/components/play/TableFelt";
import { ScopeBanner } from "@/components/preflop/PreflopResultView";
import { StrategyMatrix } from "@/components/solver/StrategyMatrix";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, Select } from "@/components/ui/Field";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

type Scope = "heads_up_push_fold" | "approximate_model";

type ModelLabel = {
  model: string;
  validated_scope: Scope;
  model_notes: string[];
  model_gap_mbb: number | null;
  iterations: number;
};

type Source = ModelLabel & {
  job_id: string;
  positions: string[];
  effective_stack_mbb: number;
  rake_bps: number;
};

export type PreflopSpot = "open" | "facing_raise" | "any";

export type PreflopDrill = {
  spot: PreflopSpot;
  path: number[];
  position: string;
  class: string;
  cards: [string, string];
  positions: string[];
  pot_mbb: number;
  invested_mbb: number[];
  live: boolean[];
  actions: { label: string; kind: string; to_mbb: number }[];
  history: { actor: string; actions: string[]; chosen: number | null }[];
};

type Dealt = ModelLabel & { job_id: string; drill: PreflopDrill; effective_stack_mbb: number };

export type PreflopGrade = ModelLabel & {
  drill: PreflopDrill;
  choice: number;
  choice_label: string;
  verdict: "matches" | "mixed" | "differs";
  your_frequency: number;
  model_action: number;
  actions: { label: string; kind: string; frequency: number }[];
  by_class: ClassStrategy[];
  mixed_min: number;
};

const SPOTS: [PreflopSpot, string][] = [
  ["any", "Any decision"],
  ["open", "Open or fold (folded to you)"],
  ["facing_raise", "Facing a raise"],
];

const VERDICT: Record<PreflopGrade["verdict"], { label: string; style: string }> = {
  matches: { label: "Matches the model", style: "bg-accent-dim text-accent-strong" },
  mixed: { label: "Mixed", style: "bg-warn-dim text-warn" },
  differs: { label: "Differs", style: "bg-danger-dim text-danger" },
};

/** A frequency for display: "<0.1%" rather than a misleading "0.0%" for tiny non-zero values. */
const pct = (x: number) => (x > 0 && x < 0.001 ? "<0.1%" : `${(x * 100).toFixed(x > 0 && x < 0.01 ? 1 : 0)}%`);

/** The decision drawn on the Play table: you at the bottom, the others in seat order. */
export function preflopTable(d: PreflopDrill, stackMbb: number): TableView {
  const lastAction = (pos: string) => {
    const step = [...d.history].reverse().find((h) => h.actor === pos && h.chosen !== null);
    return step ? step.actions[step.chosen!] : null;
  };
  const button = d.positions.includes("BTN") ? "BTN" : "SB";
  const seat = (i: number): SeatView => {
    const pos = d.positions[i];
    const hero = pos === d.position;
    return {
      name: hero ? "You" : pos,
      is_hero: hero,
      position: pos,
      is_button: pos === button,
      stack: Math.max(0, Math.round(stackMbb - d.invested_mbb[i])),
      street_commit: Math.round(d.invested_mbb[i]),
      folded: !d.live[i],
      all_in: false,
      busted: false,
      finish: null,
      cards: hero ? d.cards : null,
      in_hand: d.live[i],
      last_action: lastAction(pos),
      hidden_cards: null,
    };
  };
  const me = d.positions.indexOf(d.position);
  const order = d.positions.map((_, k) => (me + k) % d.positions.length);
  return {
    status: "awaiting_hero",
    format: "cash",
    units: "bb",
    big_blind: 1000,
    small_blind: 500,
    ante: 0,
    level: null,
    hand_no: 0,
    street: "preflop",
    board: [],
    pot: Math.round(d.pot_mbb),
    seats: order.map(seat),
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

/** "UTG fold, HJ raise 2.5, CO fold" */
function historyText(d: PreflopDrill): string {
  return d.history.map((h) => `${h.actor} ${h.actions[h.chosen!]}`).join(", ");
}

export function PreflopTrainer() {
  const sources = useApi<Source[]>("/api/v1/trainer/preflop/sources");
  const [jobId, setJobId] = useState("");
  const [spot, setSpot] = useState<PreflopSpot>("any");
  const [dealt, setDealt] = useState<Dealt | null>(null);
  const [grade, setGrade] = useState<PreflopGrade | null>(null);
  const [tally, setTally] = useState({ n: 0, matches: 0 });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function deal() {
    setBusy(true);
    setError(null);
    try {
      setDealt(await api.post<Dealt>("/api/v1/trainer/preflop/drill", { job_id: jobId || null, spot }));
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
      const g = await api.post<PreflopGrade>("/api/v1/trainer/preflop/answer", {
        job_id: dealt.job_id,
        drill: dealt.drill,
        choice,
      });
      setGrade(g);
      setTally((t) => ({ n: t.n + 1, matches: t.matches + (g.verdict === "matches" ? 1 : 0) }));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (sources.error) return <ErrorState message={sources.error.message} />;
  if (sources.loading && !sources.data) return <LoadingState label="Loading saved preflop solves…" />;
  if (!sources.data?.length) {
    return (
      <EmptyState
        title="No saved preflop solves yet"
        description="Preflop drills are dealt from your own preflop solves. Run one on the Preflop page (it takes a few minutes), then come back."
        action={
          <Link href="/preflop" className="text-sm text-accent hover:underline">
            Go to the Preflop page
          </Link>
        }
      />
    );
  }
  const d = dealt?.drill;
  const scope = dealt?.validated_scope ?? sources.data.find((s) => s.job_id === jobId)?.validated_scope;
  return (
    <div className="grid gap-6">
      <Card>
        <CardHeader
          title="Preflop drill"
          subtitle="A decision from one of your saved preflop solves, compared with what the model does"
        />
        <CardBody className="flex flex-col gap-4">
          {scope ? (
            <ScopeBanner scope={scope} />
          ) : (
            <ScopeBanner
              scope={
                sources.data.every((s) => s.validated_scope === "heads_up_push_fold")
                  ? "heads_up_push_fold"
                  : "approximate_model"
              }
            />
          )}
          <div className="grid gap-3 sm:max-w-xl sm:grid-cols-2">
            <Field label="Preflop solve">
              {(id) => (
                <Select id={id} value={jobId} onChange={(e) => setJobId(e.target.value)}>
                  <option value="">Any saved preflop solve</option>
                  {sources.data?.map((s) => (
                    <option key={s.job_id} value={s.job_id}>
                      {s.positions.length}-handed · {formatBb(s.effective_stack_mbb)}bb · rake {s.rake_bps / 100}%
                    </option>
                  ))}
                </Select>
              )}
            </Field>
            <Field label="Spot">
              {(id) => (
                <Select id={id} value={spot} onChange={(e) => setSpot(e.target.value as PreflopSpot)}>
                  {SPOTS.map(([k, label]) => (
                    <option key={k} value={k}>
                      {label}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          </div>
          {d && dealt && (
            <div className="flex flex-col gap-3" aria-label="Current preflop drill">
              <div className="w-full max-w-xl self-center">
                <TableFelt t={preflopTable(d, dealt.effective_stack_mbb)} showAll={false} />
              </div>
              <p className="text-xs text-muted">
                You are {d.position} with <CardRow cards={d.cards} size="sm" /> ({d.class}).{" "}
                {d.history.length ? `${historyText(d)}.` : "You are first to act."}
              </p>
              {!grade ? (
                <div className="flex flex-wrap gap-2" role="group" aria-label="Your action">
                  {d.actions.map((a, i) => (
                    <Button key={a.label} onClick={() => answer(i)} disabled={busy}>
                      {a.label}
                    </Button>
                  ))}
                </div>
              ) : (
                <div className="flex flex-col gap-3 rounded-md border border-line bg-surface-2/50 px-3 py-2">
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
                      The model plays {grade.choice_label} with {d.class} {pct(grade.your_frequency)} of the time
                      {grade.verdict !== "matches" &&
                        `; its most frequent play is ${grade.actions[grade.model_action].label} (${pct(grade.actions[grade.model_action].frequency)})`}
                      .
                    </span>
                  </p>
                  <ul className="flex flex-col gap-1" aria-label="The model's play with this hand">
                    {grade.actions.map((a, i) => (
                      <li key={a.label} className="flex items-center gap-2 text-xs">
                        <span
                          className={cn("w-24 shrink-0", i === grade.choice ? "font-semibold text-fg" : "text-muted")}
                        >
                          {a.label}
                          {i === grade.choice && " (you)"}
                        </span>
                        <span className="h-2 flex-1 overflow-hidden rounded bg-surface-3">
                          <span className="block h-full bg-accent" style={{ width: `${a.frequency * 100}%` }} />
                        </span>
                        <span className="w-12 text-right font-mono tabular text-muted">{pct(a.frequency)}</span>
                      </li>
                    ))}
                  </ul>
                  <details>
                    <summary className="cursor-pointer text-xs text-accent">
                      The model&apos;s whole {d.position} range here
                    </summary>
                    <div className="mt-2">
                      <StrategyMatrix actions={grade.actions.map((a) => a.label)} byClass={grade.by_class} />
                    </div>
                  </details>
                  <p className="text-[11px] text-subtle">
                    Graded by how often the model takes your action with this hand, not by EV: GTOpen&apos;s preflop
                    model gives no value for each action. &quot;Matches&quot; means it is the model&apos;s most frequent
                    play; &quot;mixed&quot; means the model still takes it at least {pct(grade.mixed_min)} of the time.{" "}
                    {grade.model}
                    {grade.model_gap_mbb !== null &&
                      `; the model's own best-response gap is ${formatBb(Math.round(grade.model_gap_mbb))}bb per hand`}
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
              <span className="font-mono text-xs text-muted tabular" aria-label="Preflop tally">
                {tally.n} answered · {Math.round((tally.matches / tally.n) * 100)}% matched the model
              </span>
            )}
          </div>
          {error && <ErrorState message={error} />}
        </CardBody>
      </Card>
    </div>
  );
}

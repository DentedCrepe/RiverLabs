"use client";

import { CircleCheck, Hammer, RotateCw } from "lucide-react";
import { useState } from "react";
import { ApiError, api } from "@/lib/api";
import {
  type Chart,
  type Drill,
  EMPTY_TALLY,
  type Grade,
  type Seat,
  type Tally,
  type TrainerStatus,
  addToTally,
  signedBb,
} from "@/lib/trainer";
import { bbToMbb, formatBb } from "@/lib/units";
import { useApi } from "@/lib/useApi";
import { CardRow } from "@/components/poker/CardText";
import { StrategyMatrix } from "@/components/solver/StrategyMatrix";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, Select, TextInput } from "@/components/ui/Field";
import { InfoTip } from "@/components/ui/InfoTip";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

export const VERDICT: Record<Grade["verdict"], { label: string; style: string }> = {
  correct: { label: "Correct", style: "bg-accent-dim text-accent-strong" },
  close: { label: "Close", style: "bg-warn-dim text-warn" },
  mistake: { label: "Mistake", style: "bg-danger-dim text-danger" },
};

function seatText(seat: Seat): string {
  return seat === "sb" ? "Small blind (button), first to act" : "Big blind, the small blind has jammed";
}

/** The exact equity table: missing (build it), building (progress), loading, or failed. */
export function TableSetup({ status, onChanged }: { status: TrainerStatus; onChanged: () => void }) {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  async function build() {
    setBusy(true);
    setError(null);
    try {
      await api.post("/api/v1/trainer/push-fold/build", {});
      onChanged();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }
  if (status.state === "loading") return <LoadingState label="Loading and checking the exact equity table…" />;
  if (status.state === "building") {
    const pct = (status.progress ?? 0) * 100;
    return (
      <div className="flex flex-col gap-2" aria-live="polite">
        <p className="text-sm text-fg">Building the exact equity table… {pct.toFixed(1)}%</p>
        <span className="h-2 overflow-hidden rounded-full bg-surface-3">
          <span className="block h-full bg-accent" style={{ width: `${pct}%` }} />
        </span>
        <p className="text-xs text-muted">
          About 47,000 exact enumerations of every board. You can leave this page; it keeps going and is saved when
          done.
        </p>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-3">
      {status.state === "error" && (
        <ErrorState title="The equity table could not be used" message={status.message ?? "unknown error"} />
      )}
      <p className="text-sm text-fg">
        The trainer grades against an exact solution, which needs a table of every hand-versus-hand all-in equity (7
        MB). It is not on this device yet.
      </p>
      <p className="text-xs text-muted">
        On Android, <code>riverlab update</code> downloads a pre-built copy, which is checked here before use (entries
        re-computed exactly). Building it here takes about 20 minutes on a 4-core computer, longer on a phone.
      </p>
      <div>
        <Button onClick={build} loading={busy} icon={<Hammer className="size-4" />}>
          Build the table here
        </Button>
      </div>
      {error && <ErrorState message={error} />}
    </div>
  );
}

function Result({ g }: { g: Grade }) {
  const v = VERDICT[g.verdict];
  const best = g.evs_bb[0] >= g.evs_bb[1] ? 0 : 1;
  const mine = g.choice === "fold" ? 0 : 1;
  return (
    <div className="flex flex-col gap-2 rounded-md border border-line bg-surface-2/50 px-3 py-2" aria-live="polite">
      <p className="flex flex-wrap items-center gap-2 text-sm">
        <span className={cn("rounded px-1.5 py-px text-xs font-semibold uppercase", v.style)}>{v.label}</span>
        {g.ev_loss_bb > 0 ? (
          <span className="text-fg">You lost {signedBb(-g.ev_loss_bb).slice(1)} against the equilibrium opponent.</span>
        ) : (
          <span className="text-fg">No EV lost.</span>
        )}
      </p>
      <table className="w-full text-xs">
        <thead>
          <tr className="text-left text-muted">
            <th className="font-normal">Action</th>
            <th className="font-normal">
              EV{" "}
              <InfoTip label="EV here">
                Expected result of the action from the start of the hand, blinds included, against the opponent playing
                the exact equilibrium. Folding the small blind is −0.5bb; folding the big blind to a jam is −1bb.
              </InfoTip>
            </th>
            <th className="font-normal">Equilibrium plays it</th>
          </tr>
        </thead>
        <tbody className="font-mono tabular">
          {g.actions.map((a, i) => (
            <tr key={a} className={cn(i === mine ? "text-fg" : "text-muted")}>
              <td className="py-0.5 font-sans capitalize">
                {a}
                {i === mine && " ← you"}
                {i === best && " ★"}
              </td>
              <td>{signedBb(g.evs_bb[i])}</td>
              <td>{(g.frequencies[i] * 100).toFixed(1)}%</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="text-[11px] text-subtle">
        ★ best action. Exact equilibrium for {g.drill.class} at {formatBb(g.drill.stack_mbb)}bb (this exact combo, card
        removal included); exploitability {g.exploitability_bb.toExponential(1)} bb/hand.
      </p>
    </div>
  );
}

function ChartView({ stackMbb }: { stackMbb: number }) {
  const { data, error, loading } = useApi<Chart>(`/api/v1/trainer/push-fold/chart?stack_mbb=${stackMbb}`);
  if (error) return <ErrorState message={error.message} />;
  if (loading || !data) return <LoadingState label="Solving this depth…" />;
  return (
    <div className="flex flex-col gap-3">
      <p className="font-mono text-xs text-muted tabular">
        {formatBb(data.stack_mbb)}bb · SB jams {(data.sb_jam_share * 100).toFixed(1)}% · BB calls{" "}
        {(data.bb_call_share * 100).toFixed(1)}% · SB EV {signedBb(data.sb_ev_bb, 4)} · exploitability{" "}
        {data.exploitability_bb.toExponential(1)} bb/hand · {data.iterations} CFR+ iterations
      </p>
      <div className="grid gap-4 lg:grid-cols-2">
        <div className="flex flex-col gap-1.5">
          <h4 className="text-xs font-semibold text-fg">Small blind</h4>
          <StrategyMatrix actions={["fold", "jam"]} byClass={data.sb} />
        </div>
        <div className="flex flex-col gap-1.5">
          <h4 className="text-xs font-semibold text-fg">Big blind facing a jam</h4>
          <StrategyMatrix actions={["fold", "call"]} byClass={data.bb} />
        </div>
      </div>
    </div>
  );
}

export function PushFoldTrainer() {
  const status = useApi<TrainerStatus>("/api/v1/trainer/push-fold", { refreshMs: 2000 });
  const ready = status.data?.state === "ready";
  const [minBb, setMinBb] = useState("5");
  const [maxBb, setMaxBb] = useState("15");
  const [seat, setSeat] = useState<"any" | Seat>("any");
  const [drill, setDrill] = useState<Drill | null>(null);
  const [grade, setGrade] = useState<Grade | null>(null);
  const [tally, setTally] = useState<Tally>(EMPTY_TALLY);
  const [chartFor, setChartFor] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function deal() {
    setError(null);
    const min = bbToMbb(minBb);
    const max = bbToMbb(maxBb);
    if (min === null || max === null) {
      setError("Enter stacks in bb, e.g. 5 and 15");
      return;
    }
    setBusy(true);
    try {
      const d = await api.post<Drill>("/api/v1/trainer/push-fold/drill", {
        min_stack_mbb: min,
        max_stack_mbb: max,
        seat: seat === "any" ? null : seat,
      });
      setDrill(d);
      setGrade(null);
      setChartFor(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function answer(choice: "fold" | "play") {
    if (!drill) return;
    setError(null);
    setBusy(true);
    try {
      const g = await api.post<Grade>("/api/v1/trainer/push-fold/answer", { drill, choice });
      setGrade(g);
      setTally((t) => addToTally(t, g));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid gap-6">
      <p className="flex items-start gap-2 rounded-md border border-accent/30 bg-accent-dim px-3 py-2 text-sm text-accent-strong">
        <CircleCheck aria-hidden className="mt-0.5 size-4 shrink-0" />
        <span>
          Heads-up push/fold, graded against RiverLab&apos;s exact equilibrium: all 1,326 hands with card removal, exact
          all-in equities, checked by exact best response. Blinds 0.5/1, equal stacks, <strong>no ante, no rake</strong>
          , which is the only preflop game RiverLab has validated.
        </span>
      </p>
      <Card>
        <CardHeader title="Push/fold drill" subtitle="Jam or fold in the small blind; call or fold against a jam" />
        <CardBody className="flex flex-col gap-4">
          {status.error ? (
            <ErrorState message={status.error.message} />
          ) : !status.data ? (
            <LoadingState label="Checking the trainer…" />
          ) : !ready ? (
            <TableSetup status={status.data} onChanged={status.reload} />
          ) : (
            <>
              <div className="grid grid-cols-3 gap-3 sm:max-w-md">
                <Field label="Stack from (bb)">
                  {(id) => <TextInput id={id} value={minBb} onChange={(e) => setMinBb(e.target.value)} />}
                </Field>
                <Field label="to (bb)">
                  {(id) => <TextInput id={id} value={maxBb} onChange={(e) => setMaxBb(e.target.value)} />}
                </Field>
                <Field label="Seat">
                  {(id) => (
                    <Select id={id} value={seat} onChange={(e) => setSeat(e.target.value as "any" | Seat)}>
                      <option value="any">Both</option>
                      <option value="sb">Small blind</option>
                      <option value="bb">Big blind</option>
                    </Select>
                  )}
                </Field>
              </div>
              <p className="text-[11px] text-subtle">Stacks from 2bb to 25bb in steps of 0.5bb.</p>
              {drill && (
                <div className="flex flex-col gap-3" aria-label="Current drill">
                  <p className="font-mono text-xs text-muted tabular">
                    {formatBb(drill.stack_mbb)}bb effective · {seatText(drill.seat)}
                  </p>
                  <div className="flex items-center gap-3">
                    <CardRow cards={drill.cards} />
                    <span className="text-sm text-muted">{drill.class}</span>
                  </div>
                  {!grade && (
                    <div className="flex gap-2">
                      <Button onClick={() => answer("fold")} disabled={busy}>
                        Fold
                      </Button>
                      <Button variant="primary" onClick={() => answer("play")} disabled={busy}>
                        {drill.seat === "sb" ? "Jam" : "Call"}
                      </Button>
                    </div>
                  )}
                  {busy && !grade && <LoadingState label="Grading (a new depth is solved first, a few seconds)…" />}
                  {grade && <Result g={grade} />}
                </div>
              )}
              <div className="flex flex-wrap gap-2">
                <Button
                  variant={drill && !grade ? "secondary" : "primary"}
                  onClick={deal}
                  loading={busy && !drill}
                  icon={<RotateCw className="size-4" />}
                >
                  {drill ? "Next hand" : "Deal"}
                </Button>
                {grade && (
                  <Button variant="ghost" onClick={() => setChartFor(chartFor === null ? grade.drill.stack_mbb : null)}>
                    {chartFor === null ? "Show the chart for this depth" : "Hide the chart"}
                  </Button>
                )}
              </div>
              {error && <ErrorState message={error} />}
              {chartFor !== null && <ChartView stackMbb={chartFor} />}
            </>
          )}
        </CardBody>
      </Card>
      {tally.hands > 0 && (
        <Card>
          <CardHeader title="This session" />
          <CardBody>
            <dl className="grid grid-cols-2 gap-2 font-mono text-sm tabular sm:grid-cols-5" aria-label="Session tally">
              {[
                ["Hands", String(tally.hands)],
                ["Correct", String(tally.correct)],
                ["Close", String(tally.close)],
                ["Mistakes", String(tally.mistakes)],
                ["EV lost / hand", `${(tally.evLost / tally.hands).toFixed(3)}bb`],
              ].map(([k, v]) => (
                <div key={k} className="rounded-md border border-line px-3 py-2">
                  <dt className="label-caps">{k}</dt>
                  <dd className="text-fg">{v}</dd>
                </div>
              ))}
            </dl>
          </CardBody>
        </Card>
      )}
    </div>
  );
}

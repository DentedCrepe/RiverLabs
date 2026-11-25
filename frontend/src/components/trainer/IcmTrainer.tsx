"use client";

import { RotateCw, TriangleAlert } from "lucide-react";
import { useState } from "react";
import { ApiError, api } from "@/lib/api";
import { type IcmDrill, type IcmGrade, type TrainerStatus } from "@/lib/trainer";
import { bbToMbb } from "@/lib/units";
import { useApi } from "@/lib/useApi";
import { CardRow } from "@/components/poker/CardText";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, Select, TextInput } from "@/components/ui/Field";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { TableSetup, VERDICT } from "./PushFoldTrainer";

const bb = (chips: number, bigBlind: number) => `${(chips / bigBlind).toFixed(1).replace(/\.0$/, "")}bb`;

/** The spot: every stack in action order, who is all-in, and the payouts. */
export function IcmSpotView({ d }: { d: IcmDrill }) {
  const s = d.spot;
  return (
    <div className="flex flex-col gap-1">
      <ol className="flex flex-wrap gap-2 text-xs" aria-label="Stacks in order of action">
        {s.stacks.map((st, k) => (
          <li
            key={k}
            className={cn(
              "rounded-md border px-2 py-1 font-mono tabular",
              k === d.hero ? "border-accent text-accent-strong" : "border-line text-muted",
            )}
          >
            {d.positions[k]} {bb(st, s.big_blind)}
            {k === d.hero && " (you)"}
            {d.facing === k && " · all-in"}
            {d.facing !== null && k > d.facing && k < d.hero && " · folded"}
            {d.facing === null && k < d.hero && " · folded"}
          </li>
        ))}
      </ol>
      <p className="text-[11px] text-subtle">
        Pays {s.payouts.join("/")}% · blinds 0.5/1bb{s.ante > 0 && ` · ante ${bb(s.ante, s.big_blind)}`}
      </p>
    </div>
  );
}

export function IcmResult({ g }: { g: IcmGrade }) {
  const v = VERDICT[g.verdict];
  return (
    <div className="flex flex-col gap-2 rounded-md border border-line p-3" aria-label="ICM grade">
      <div className="flex flex-wrap items-center gap-2">
        <span className={cn("rounded px-1.5 py-px text-xs font-semibold uppercase", v.style)}>{v.label}</span>
        <span className="text-sm text-fg">
          {g.loss_pct > 0 ? `Cost ${g.loss_pct.toFixed(2)}% of the prize pool` : "No cost"}
        </span>
      </div>
      <table className="w-full max-w-sm text-sm">
        <thead className="text-left text-xs text-muted">
          <tr>
            <th className="font-normal">Action</th>
            <th className="font-normal">Your prize-pool share</th>
            <th className="font-normal">Solution plays it</th>
          </tr>
        </thead>
        <tbody className="font-mono tabular">
          {g.actions.map((a, i) => (
            <tr key={a} className={cn(g.choice === (i === 1 ? "play" : "fold") && "text-accent-strong")}>
              <td className="capitalize">{a}</td>
              <td>{g.values_pct[i].toFixed(2)}%</td>
              <td>{(g.frequencies[i] * 100).toFixed(0)}%</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="text-[11px] text-subtle">
        A modelled approximation: within {g.epsilon_pct.toFixed(4)}% of the prize pool of an equilibrium of the model (
        {g.iterations} CFR+ iterations).
      </p>
      <ul className="list-disc pl-5 text-[11px] text-subtle">
        {g.approximations.map((a) => (
          <li key={a}>{a}</li>
        ))}
      </ul>
    </div>
  );
}

export function IcmTrainer() {
  const status = useApi<TrainerStatus>("/api/v1/trainer/push-fold", { refreshMs: 2000 });
  const [players, setPlayers] = useState("4");
  const [minBb, setMinBb] = useState("5");
  const [maxBb, setMaxBb] = useState("15");
  const [payouts, setPayouts] = useState("50/30/20");
  const [drill, setDrill] = useState<IcmDrill | null>(null);
  const [grade, setGrade] = useState<IcmGrade | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function deal() {
    setError(null);
    const min = bbToMbb(minBb);
    const max = bbToMbb(maxBb);
    const paid = payouts.trim() ? payouts.split(/[/,]/).map((x) => Number(x.trim())) : [];
    if (min === null || max === null) return setError("Enter stacks in bb, e.g. 5 and 15");
    if (paid.some((p) => !Number.isInteger(p) || p <= 0))
      return setError("Payouts are whole percentages, e.g. 50/30/20");
    setBusy(true);
    try {
      const d = await api.post<IcmDrill>("/api/v1/trainer/push-fold/icm/drill", {
        players: Number(players),
        min_stack_mbb: min,
        max_stack_mbb: max,
        payouts: paid,
      });
      setDrill(d);
      setGrade(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function answer(choice: "fold" | "play") {
    if (!drill) return;
    setBusy(true);
    setError(null);
    try {
      setGrade(await api.post<IcmGrade>("/api/v1/trainer/push-fold/icm/answer", { drill, choice }));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid gap-6">
      <p className="flex items-start gap-2 rounded-md border border-warn/40 bg-warn-dim/30 px-3 py-2 text-sm text-fg">
        <TriangleAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-warn" />
        <span>
          Tournament push/fold for 3 to 6 players, valued with ICM for your payouts. A{" "}
          <strong>modelled approximation</strong>: only all-in or fold, no overcalls, and the folded players&apos; cards
          ignored. Each spot is solved and shown with how close it is to an equilibrium of that model.
        </span>
      </p>
      <Card>
        <CardHeader title="ICM push/fold drill" subtitle="Jam or fold first in; call or fold against an all-in" />
        <CardBody className="flex flex-col gap-4">
          {status.error ? (
            <ErrorState message={status.error.message} />
          ) : !status.data ? (
            <LoadingState label="Checking the trainer…" />
          ) : status.data.state !== "ready" ? (
            <TableSetup status={status.data} onChanged={status.reload} />
          ) : (
            <>
              <div className="grid grid-cols-2 gap-3 sm:max-w-xl sm:grid-cols-4">
                <Field label="Players">
                  {(id) => (
                    <Select id={id} value={players} onChange={(e) => setPlayers(e.target.value)}>
                      {[3, 4, 5, 6].map((n) => (
                        <option key={n} value={n}>
                          {n}
                        </option>
                      ))}
                    </Select>
                  )}
                </Field>
                <Field label="Stacks from (bb)">
                  {(id) => <TextInput id={id} value={minBb} onChange={(e) => setMinBb(e.target.value)} />}
                </Field>
                <Field label="to (bb)">
                  {(id) => <TextInput id={id} value={maxBb} onChange={(e) => setMaxBb(e.target.value)} />}
                </Field>
                <Field label="Payouts (%)" hint="Empty: the default for the table size">
                  {(id) => <TextInput id={id} value={payouts} onChange={(e) => setPayouts(e.target.value)} />}
                </Field>
              </div>
              {drill && (
                <div className="flex flex-col gap-3" aria-label="Current ICM drill">
                  <IcmSpotView d={drill} />
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
                        {drill.facing === null ? "Jam" : "Call"}
                      </Button>
                    </div>
                  )}
                  {grade && <IcmResult g={grade} />}
                </div>
              )}
              <div>
                <Button
                  variant={drill && !grade ? "secondary" : "primary"}
                  onClick={deal}
                  loading={busy && (!drill || Boolean(grade))}
                  icon={<RotateCw className="size-4" />}
                >
                  {drill ? "Next spot" : "Deal"}
                </Button>
              </div>
              {busy && !drill && <LoadingState label="Solving the first spot (a few seconds)…" />}
              {error && <ErrorState message={error} />}
            </>
          )}
        </CardBody>
      </Card>
    </div>
  );
}

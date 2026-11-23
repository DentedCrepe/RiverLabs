"use client";

import { useState } from "react";
import { type PlayReport, amount } from "@/lib/play";
import { formatBb } from "@/lib/units";
import { useApi } from "@/lib/useApi";
import { ReportView } from "@/components/play/PlayReport";
import { LeaksSection } from "./Leaks";
import { Metric } from "@/components/solver/ResultView";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

type Scope = "cash" | "sit_and_go";

type PlayAnalytics = {
  scope: Scope;
  sessions: { id: string; format: string; status: string; created_at: string; hands: number; hero_net: number }[];
  report: PlayReport | null;
};

type Summary = { answers: number; correct: number; close: number; mistakes: number; ev_lost_bb: number };
type TrainerAnalytics = {
  total: Summary;
  by_seat: { sb: Summary; bb: Summary };
  by_stack: (Summary & { min_mbb: number; max_mbb: number })[];
  costliest: (Summary & { hand_class: string; seat: "sb" | "bb" })[];
  recent: {
    created_at: string;
    stack_mbb: number;
    seat: "sb" | "bb";
    cards: string;
    hand_class: string;
    choice: "fold" | "play";
    verdict: "correct" | "close" | "mistake";
    ev_loss_bb: number;
  }[];
};

const FORMAT: Record<string, string> = { cash: "Cash", fast_fold: "Fast-fold", sit_and_go: "Sit & go" };
const pctOf = (n: number, d: number) => (d ? `${((n / d) * 100).toFixed(0)}%` : "—");
const perAnswer = (s: Summary) => (s.answers ? `${(s.ev_lost_bb / s.answers).toFixed(3)}bb` : "—");

function PlaySection() {
  const [scope, setScope] = useState<Scope>("cash");
  const { data, error, loading } = useApi<PlayAnalytics>(`/api/v1/analytics/play?scope=${scope}`);
  const units = scope === "cash" ? "bb" : "chips";
  return (
    <Card>
      <CardHeader title="Games against the bots" subtitle="Every hand you played, as one report" />
      <CardBody className="flex flex-col gap-4">
        <div role="tablist" aria-label="Game type" className="flex gap-1">
          {(
            [
              ["cash", "Cash & fast-fold"],
              ["sit_and_go", "Sit & go"],
            ] as const
          ).map(([k, label]) => (
            <button
              key={k}
              role="tab"
              type="button"
              aria-selected={scope === k}
              onClick={() => setScope(k)}
              className={cn(
                "rounded-md px-3 py-1 text-sm",
                scope === k ? "bg-surface-3 text-fg" : "text-muted hover:bg-surface-2",
              )}
            >
              {label}
            </button>
          ))}
        </div>
        {error ? (
          <ErrorState message={error.message} />
        ) : loading && !data ? (
          <LoadingState label="Loading your hands…" />
        ) : !data || !data.report ? (
          <EmptyState title="No hands yet" description="Play a game on the Play page; its hands are counted here." />
        ) : (
          <>
            <ul className="flex flex-col gap-1 text-xs" aria-label="Sessions">
              {data.sessions.map((s) => (
                <li key={s.id} className="flex flex-wrap gap-x-3 font-mono text-muted tabular">
                  <span className="w-40">{new Date(s.created_at).toLocaleString()}</span>
                  <span className="w-20 font-sans text-fg">{FORMAT[s.format] ?? s.format}</span>
                  <span className="w-20">
                    {s.hands} hand{s.hands === 1 ? "" : "s"}
                  </span>
                  <span className={s.hero_net >= 0 ? "text-accent-strong" : "text-danger"}>
                    {s.hero_net >= 0 ? "+" : "−"}
                    {amount(Math.abs(s.hero_net), units)}
                  </span>
                </li>
              ))}
            </ul>
            <ReportView report={data.report} />
          </>
        )}
      </CardBody>
    </Card>
  );
}

function SummaryRow({ label, s }: { label: string; s: Summary }) {
  return (
    <tr className="font-mono tabular">
      <td className="py-0.5 pr-3 font-sans text-fg">{label}</td>
      <td className="pr-3">{s.answers}</td>
      <td className="pr-3">{pctOf(s.correct, s.answers)}</td>
      <td className="pr-3">{s.mistakes}</td>
      <td>{perAnswer(s)}</td>
    </tr>
  );
}

function TrainerSection() {
  const { data, error, loading } = useApi<TrainerAnalytics>("/api/v1/analytics/trainer");
  return (
    <Card>
      <CardHeader
        title="Push/fold trainer"
        subtitle="Your answers, graded by EV against the exact heads-up equilibrium"
      />
      <CardBody className="flex flex-col gap-4">
        {error ? (
          <ErrorState message={error.message} />
        ) : loading && !data ? (
          <LoadingState label="Loading your answers…" />
        ) : !data || data.total.answers === 0 ? (
          <EmptyState title="No answers yet" description="Drills on the Trainer page are counted here." />
        ) : (
          <>
            <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
              <Metric label="Answers" value={String(data.total.answers)} />
              <Metric
                label="Correct"
                value={pctOf(data.total.correct, data.total.answers)}
                sub={`${data.total.close} close · ${data.total.mistakes} mistakes`}
              />
              <Metric label="EV lost / answer" value={perAnswer(data.total)} sub="against the equilibrium" />
              <Metric label="EV lost in total" value={`${data.total.ev_lost_bb.toFixed(2)}bb`} />
            </div>
            <div className="overflow-x-auto">
              <table className="text-xs text-muted" aria-label="Trainer results by seat and stack">
                <thead>
                  <tr className="text-left">
                    <th className="pr-3 font-normal" />
                    <th className="pr-3 font-normal">Answers</th>
                    <th className="pr-3 font-normal">Correct</th>
                    <th className="pr-3 font-normal">Mistakes</th>
                    <th className="font-normal">EV lost / answer</th>
                  </tr>
                </thead>
                <tbody>
                  <SummaryRow label="Small blind (jam or fold)" s={data.by_seat.sb} />
                  <SummaryRow label="Big blind (call or fold)" s={data.by_seat.bb} />
                  {data.by_stack
                    .filter((b) => b.answers > 0)
                    .map((b) => (
                      <SummaryRow key={b.min_mbb} label={`${formatBb(b.min_mbb)}–${formatBb(b.max_mbb)}bb`} s={b} />
                    ))}
                </tbody>
              </table>
            </div>
            {data.costliest.length > 0 && (
              <div className="flex flex-col gap-1.5">
                <h4 className="text-xs font-semibold text-fg">Hands that cost you the most</h4>
                <ul className="flex flex-col gap-0.5 font-mono text-xs text-muted tabular" aria-label="Costliest hands">
                  {data.costliest.map((c) => (
                    <li key={`${c.hand_class}-${c.seat}`} className="flex gap-3">
                      <span className="w-10 text-fg">{c.hand_class}</span>
                      <span className="w-6 uppercase">{c.seat}</span>
                      <span className="w-24">{c.ev_lost_bb.toFixed(3)}bb lost</span>
                      <span>
                        {c.mistakes} of {c.answers} wrong
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </>
        )}
      </CardBody>
    </Card>
  );
}

type PostflopAnalytics = {
  total: Summary;
  by_street: (Summary & { street: string })[];
  by_position: { oop: Summary; ip: Summary };
  costliest: (Summary & { hand_class: string; street: string })[];
};

function PostflopSection() {
  const { data, error, loading } = useApi<PostflopAnalytics>("/api/v1/analytics/postflop");
  return (
    <Card>
      <CardHeader title="Postflop drills" subtitle="Your answers, graded by EV from the saved solves they came from" />
      <CardBody className="flex flex-col gap-4">
        {error ? (
          <ErrorState message={error.message} />
        ) : loading && !data ? (
          <LoadingState label="Loading your answers…" />
        ) : !data || data.total.answers === 0 ? (
          <EmptyState title="No answers yet" description="Postflop drills on the Trainer page are counted here." />
        ) : (
          <>
            <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
              <Metric label="Answers" value={String(data.total.answers)} />
              <Metric
                label="Correct"
                value={pctOf(data.total.correct, data.total.answers)}
                sub={`${data.total.close} close · ${data.total.mistakes} mistakes`}
              />
              <Metric label="EV lost / answer" value={perAnswer(data.total)} sub="against the solve" />
              <Metric label="EV lost in total" value={`${data.total.ev_lost_bb.toFixed(2)}bb`} />
            </div>
            <div className="overflow-x-auto">
              <table className="text-xs text-muted" aria-label="Postflop results by street and position">
                <thead>
                  <tr className="text-left">
                    <th className="pr-3 font-normal" />
                    <th className="pr-3 font-normal">Answers</th>
                    <th className="pr-3 font-normal">Correct</th>
                    <th className="pr-3 font-normal">Mistakes</th>
                    <th className="font-normal">EV lost / answer</th>
                  </tr>
                </thead>
                <tbody>
                  {data.by_street
                    .filter((s) => s.answers > 0)
                    .map((s) => (
                      <SummaryRow key={s.street} label={s.street[0].toUpperCase() + s.street.slice(1)} s={s} />
                    ))}
                  <SummaryRow label="Out of position" s={data.by_position.oop} />
                  <SummaryRow label="In position" s={data.by_position.ip} />
                </tbody>
              </table>
            </div>
            {data.costliest.length > 0 && (
              <ul
                className="flex flex-col gap-0.5 font-mono text-xs text-muted tabular"
                aria-label="Costliest postflop hands"
              >
                {data.costliest.map((c) => (
                  <li key={`${c.hand_class}-${c.street}`} className="flex gap-3">
                    <span className="w-10 text-fg">{c.hand_class}</span>
                    <span className="w-10">{c.street}</span>
                    <span className="w-24">{c.ev_lost_bb.toFixed(2)}bb lost</span>
                    <span>
                      {c.mistakes} of {c.answers} wrong
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </CardBody>
    </Card>
  );
}

export function Analytics() {
  return (
    <div className="grid gap-6">
      <PlaySection />
      <LeaksSection />
      <TrainerSection />
      <PostflopSection />
    </div>
  );
}

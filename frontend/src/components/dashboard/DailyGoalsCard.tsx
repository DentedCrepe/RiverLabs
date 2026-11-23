"use client";

import { Flame, Pencil } from "lucide-react";
import Link from "next/link";
import { type FormEvent, useState } from "react";
import { ApiError, api } from "@/lib/api";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, TextInput } from "@/components/ui/Field";
import { InfoTip } from "@/components/ui/InfoTip";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

export type DailyPlan = {
  today: { date: string; hands: number; study: number; done: boolean };
  targets: { hands: number; study: number };
  streak: number;
  best_streak: number;
  days: { date: string; hands: number; study: number; met: boolean }[];
  recommended: {
    kind: string;
    title: string;
    reason: string;
    href: string;
    also?: { href: string; label: string };
  }[];
};

function Progress({ label, value, target, info }: { label: string; value: number; target: number; info?: string }) {
  const pct = Math.min(100, (value / target) * 100);
  return (
    <div className="flex flex-col gap-1">
      <p className="flex items-center justify-between text-xs">
        <span className="flex items-center gap-1 text-muted">
          {label}
          {info && <InfoTip label={label}>{info}</InfoTip>}
        </span>
        <span className={cn("font-mono tabular", value >= target ? "text-accent-strong" : "text-fg")}>
          {value} / {target}
        </span>
      </p>
      <span className="h-2 overflow-hidden rounded-full bg-surface-3">
        <span className={cn("block h-full", value >= target ? "bg-accent" : "bg-info")} style={{ width: `${pct}%` }} />
      </span>
    </div>
  );
}

function Targets({ plan, onSaved }: { plan: DailyPlan; onSaved: (p: DailyPlan) => void }) {
  const [hands, setHands] = useState(String(plan.targets.hands));
  const [study, setStudy] = useState(String(plan.targets.study));
  const [error, setError] = useState<string | null>(null);
  async function save(e: FormEvent) {
    e.preventDefault();
    const h = Number(hands);
    const s = Number(study);
    if (!Number.isInteger(h) || !Number.isInteger(s) || h < 1 || s < 1 || h > 1000 || s > 1000) {
      setError("Whole numbers from 1 to 1000");
      return;
    }
    try {
      onSaved(await api.put<DailyPlan>("/api/v1/goals", { hands_target: h, study_target: s }));
    } catch (err) {
      setError(err instanceof ApiError ? err.message : String(err));
    }
  }
  return (
    <form onSubmit={save} className="flex flex-wrap items-end gap-2" aria-label="Daily targets">
      <Field label="Hands a day">
        {(id) => (
          <TextInput
            id={id}
            inputMode="numeric"
            className="w-24"
            value={hands}
            onChange={(e) => setHands(e.target.value)}
          />
        )}
      </Field>
      <Field label="Study items a day">
        {(id) => (
          <TextInput
            id={id}
            inputMode="numeric"
            className="w-24"
            value={study}
            onChange={(e) => setStudy(e.target.value)}
          />
        )}
      </Field>
      <Button type="submit" size="sm" variant="primary">
        Save
      </Button>
      {error && <p className="w-full text-xs text-danger">{error}</p>}
    </form>
  );
}

/** Today's targets, the streak, the last two weeks and what to study. */
export function DailyGoalsCard() {
  const { data, error, loading, reload } = useApi<DailyPlan>("/api/v1/goals/today");
  const [editing, setEditing] = useState(false);
  const [saved, setSaved] = useState<DailyPlan | null>(null);
  const p = saved ?? data;
  return (
    <Card>
      <CardHeader
        title="Today"
        icon={<Flame className={cn("size-4", p?.today.done ? "text-warn" : "text-subtle")} />}
        actions={
          <Button size="sm" variant="ghost" onClick={() => setEditing(!editing)} icon={<Pencil className="size-3.5" />}>
            Targets
          </Button>
        }
      />
      <CardBody className="flex flex-col gap-4">
        {error ? (
          <ErrorState message={error.message} />
        ) : loading && !p ? (
          <LoadingState />
        ) : p ? (
          <>
            <p className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
              <span className="text-2xl font-semibold text-fg tabular">{p.streak}</span>
              <span className="text-sm text-muted">
                day streak{p.today.done ? " · today's goals done" : ""} · best {p.best_streak}
              </span>
            </p>
            {editing && (
              <Targets
                plan={p}
                onSaved={(n) => {
                  setSaved(n);
                  setEditing(false);
                  reload();
                }}
              />
            )}
            <div className="grid gap-3 sm:grid-cols-2">
              <Progress label="Hands played" value={p.today.hands} target={p.targets.hands} />
              <Progress
                label="Study"
                value={p.today.study}
                target={p.targets.study}
                info="Academy questions answered, push/fold drills and postflop drills each count as one."
              />
            </div>
            <div className="flex gap-1" aria-label="Last 14 days">
              {p.days.map((d) => (
                <span
                  key={d.date}
                  title={`${d.date}: ${d.hands} hands, ${d.study} study${d.met ? " · goals met" : ""}`}
                  className={cn(
                    "h-4 flex-1 rounded-sm",
                    d.met ? "bg-accent" : d.hands + d.study > 0 ? "bg-accent/30" : "bg-surface-3",
                  )}
                />
              ))}
            </div>
            {p.recommended.length > 0 && (
              <div className="flex flex-col gap-2">
                <h4 className="text-xs font-semibold text-fg">Recommended today</h4>
                <ul className="flex flex-col gap-2" aria-label="Recommended today">
                  {p.recommended.map((r) => (
                    <li key={r.kind + r.title} className="flex flex-col gap-0.5 text-xs">
                      <span className="flex flex-wrap items-center gap-2">
                        <Link href={r.href} className="font-medium text-accent hover:text-accent-strong">
                          {r.title}
                        </Link>
                        {r.also && (
                          <Link href={r.also.href} className="text-muted hover:text-fg">
                            {r.also.label}
                          </Link>
                        )}
                      </span>
                      <span className="text-muted">{r.reason}</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </>
        ) : null}
      </CardBody>
    </Card>
  );
}

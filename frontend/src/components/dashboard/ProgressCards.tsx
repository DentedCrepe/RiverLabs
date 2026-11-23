"use client";

import { BarChart3, GraduationCap } from "lucide-react";
import Link from "next/link";
import type { PlayReport } from "@/lib/play";
import { useApi } from "@/lib/useApi";
import { Metric } from "@/components/solver/ResultView";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";

type Summary = { answers: number; correct: number; mistakes: number; ev_lost_bb: number };

const link = (href: string, label: string) => (
  <Link href={href} className="text-xs text-accent hover:text-accent-strong">
    {label}
  </Link>
);

/** Push/fold drill results (from stored, engine-graded answers). */
export function TrainingProgressCard() {
  const { data, error, loading } = useApi<{ total: Summary }>("/api/v1/analytics/trainer");
  const t = data?.total;
  return (
    <Card>
      <CardHeader
        title="Training progress"
        icon={<GraduationCap className="size-4" />}
        actions={link("/trainer", "Open trainer")}
      />
      <CardBody>
        {error ? (
          <ErrorState message={error.message} />
        ) : loading && !data ? (
          <LoadingState />
        ) : !t || t.answers === 0 ? (
          <EmptyState
            title="No drills yet"
            description="Push/fold drills on the Trainer page are graded and counted here."
          />
        ) : (
          <div className="grid grid-cols-3 gap-2">
            <Metric label="Drills" value={String(t.answers)} />
            <Metric label="Correct" value={`${((t.correct / t.answers) * 100).toFixed(0)}%`} />
            <Metric label="EV lost" value={`${(t.ev_lost_bb / t.answers).toFixed(3)}bb`} sub="per drill" />
          </div>
        )}
      </CardBody>
    </Card>
  );
}

/** Results of your cash and fast-fold games (the engine's report over every hand). */
export function PerformanceCard() {
  const { data, error, loading } = useApi<{ report: PlayReport | null }>("/api/v1/analytics/play?scope=cash");
  const r = data?.report;
  return (
    <Card>
      <CardHeader
        title="Performance"
        icon={<BarChart3 className="size-4" />}
        actions={link("/analytics", "Analytics")}
      />
      <CardBody>
        {error ? (
          <ErrorState message={error.message} />
        ) : loading && !data ? (
          <LoadingState />
        ) : !r ? (
          <EmptyState title="No hands yet" description="Cash and fast-fold games on the Play page are counted here." />
        ) : (
          <div className="grid grid-cols-3 gap-2">
            <Metric label="Hands" value={String(r.hands)} />
            <Metric label="Result" value={`${r.net_bb >= 0 ? "+" : "−"}${Math.abs(r.net_bb).toFixed(1)}bb`} />
            <Metric
              label="bb / 100"
              value={r.bb_per_100 === null ? "—" : r.bb_per_100.toFixed(1)}
              sub={r.caveats.some((c) => c.includes("small sample")) ? "small sample" : undefined}
            />
          </div>
        )}
      </CardBody>
    </Card>
  );
}

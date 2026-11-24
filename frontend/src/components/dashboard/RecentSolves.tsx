"use client";

import { Cpu } from "lucide-react";
import Link from "next/link";
import type { SolverJob } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { jobPollMs } from "@/lib/solverJobs";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";

export function RecentSolves() {
  const { data, error, loading } = useApi<SolverJob[]>("/api/v1/solver/jobs?limit=5", {
    refreshMs: (jobs) => (jobPollMs(jobs) === 1500 ? 5000 : 30_000),
  });
  return (
    <Card>
      <CardHeader
        title="Recent solves"
        icon={<Cpu className="size-4" />}
        actions={
          <Link href="/solver" className="text-xs text-accent hover:text-accent-strong">
            Open solver
          </Link>
        }
      />
      <CardBody>
        {error ? (
          <ErrorState message={error.message} />
        ) : loading && !data ? (
          <LoadingState />
        ) : !data || data.length === 0 ? (
          <EmptyState title="No solves yet" description="Queue a heads-up postflop solve from the Solver page." />
        ) : (
          <ul className="divide-y divide-line text-sm">
            {data.map((j) => (
              <li key={j.id} className="flex items-center justify-between gap-2 py-2">
                <span className="text-fg capitalize">{j.status}</span>
                <span className="font-mono text-xs text-muted tabular">
                  {j.iterations.toLocaleString()} it
                  {j.progress.exploitability_pct_pot !== undefined &&
                    ` · ${j.progress.exploitability_pct_pot.toFixed(3)}% pot`}
                </span>
              </li>
            ))}
          </ul>
        )}
      </CardBody>
    </Card>
  );
}

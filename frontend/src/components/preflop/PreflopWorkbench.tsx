"use client";

import { useState } from "react";
import type { SolverJob } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { JobList } from "@/components/solver/JobList";
import { PreflopForm } from "./PreflopForm";
import { PreflopResultView } from "./PreflopResultView";

export function PreflopWorkbench() {
  const jobs = useApi<SolverJob[]>("/api/v1/solver/jobs?kind=preflop&limit=20", { refreshMs: 1500 });
  const [selected, setSelected] = useState<string | null>(null);
  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
      <Card>
        <CardHeader title="New preflop solve" subtitle="Approximate model; heads-up push/fold is validated" />
        <CardBody>
          <PreflopForm onSubmitted={() => jobs.reload()} />
        </CardBody>
      </Card>
      <Card>
        <CardHeader title="Preflop solves" subtitle="Shares the solver queue with postflop solves" />
        <CardBody className="py-1">
          <JobList
            jobs={jobs.data}
            error={jobs.error}
            loading={jobs.loading}
            selected={selected}
            onSelect={setSelected}
            onChanged={jobs.reload}
          />
        </CardBody>
      </Card>
      {selected && (
        <Card className="xl:col-span-2">
          <CardHeader title="Result" />
          <CardBody>
            <PreflopResultView key={selected} jobId={selected} />
          </CardBody>
        </Card>
      )}
    </div>
  );
}

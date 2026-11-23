"use client";

import { useState } from "react";
import type { SolverJob } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { JobList } from "./JobList";
import { ResultView } from "./ResultView";
import { SolveForm } from "./SolveForm";

export function SolverWorkbench() {
  const jobs = useApi<SolverJob[]>("/api/v1/solver/jobs?limit=20", { refreshMs: 1500 });
  const [selected, setSelected] = useState<string | null>(null);
  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
      <Card>
        <CardHeader title="New postflop solve" subtitle="Heads-up; flop, turn or river root" />
        <CardBody>
          <SolveForm onSubmitted={() => jobs.reload()} />
        </CardBody>
      </Card>
      <Card>
        <CardHeader title="Solves" subtitle="One solve runs at a time; others wait in the queue" />
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
            <ResultView key={selected} jobId={selected} />
          </CardBody>
        </Card>
      )}
    </div>
  );
}

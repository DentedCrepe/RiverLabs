"use client";

import { CircleAlert, Cpu } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { Suspense, useState } from "react";
import type { SpotExport } from "@/lib/play";
import { formFromSpec } from "@/lib/solve";
import type { SolverJob } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { CardRow } from "@/components/poker/CardText";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { JobList } from "./JobList";
import type { Follow } from "./NodeBrowser";
import { ResultView } from "./ResultView";
import { SolveForm } from "./SolveForm";

type SpotQuery = { game: string; hand: string; decision: string };

/** What "Solve this spot" sent over: the spot, its approximations, and the form pre-filled. */
function SpotBanner({ spot }: { spot: SpotExport }) {
  return (
    <div className="flex flex-col gap-2 rounded-md border border-accent/30 bg-accent-dim/40 px-3 py-2">
      <p className="flex flex-wrap items-center gap-2 text-sm font-medium text-fg">
        <Cpu aria-hidden className="size-4 text-accent" />
        From your hand: <CardRow cards={spot.hero_cards} size="sm" /> on the {spot.street},{" "}
        {spot.hero_player === 0 ? "out of position" : "in position"}
      </p>
      <p className="text-xs text-muted">
        Everything below is editable. When the solve finishes, its result opens and walks the real line to your
        decision.
      </p>
      <ul className="flex flex-col gap-1">
        {spot.notes.map((n) => (
          <li key={n} className="flex items-start gap-2 text-xs text-warn">
            <CircleAlert aria-hidden className="mt-px size-3.5 shrink-0" />
            {n}
          </li>
        ))}
      </ul>
    </div>
  );
}

function Workbench({ query }: { query: SpotQuery | null }) {
  const jobs = useApi<SolverJob[]>("/api/v1/solver/jobs?kind=postflop&limit=20", { refreshMs: 1500 });
  const spotUrl = query
    ? `/api/v1/play/sessions/${encodeURIComponent(query.game)}/hands/${encodeURIComponent(query.hand)}/spot?decision=${encodeURIComponent(query.decision)}`
    : null;
  const spot = useApi<SpotExport>(spotUrl);
  const [selected, setSelected] = useState<string | null>(null);
  // Jobs queued from this spot follow its line when opened.
  const [spotJobs, setSpotJobs] = useState<Set<string>>(new Set());
  // The latest solve queued from the spot opens by itself when it finishes.
  const [awaiting, setAwaiting] = useState<string | null>(null);
  const awaitedStatus = awaiting ? jobs.data?.find((j) => j.id === awaiting)?.status : undefined;
  if (awaiting && awaitedStatus && awaitedStatus !== "queued" && awaitedStatus !== "running") {
    setAwaiting(null);
    if (awaitedStatus === "succeeded") setSelected(awaiting);
  }
  const loadedSpot = query ? spot.data : null;
  const follow: Follow | undefined =
    loadedSpot && selected && spotJobs.has(selected)
      ? {
          line: loadedSpot.line,
          heroPlayer: loadedSpot.hero_player,
          heroAction: loadedSpot.hero_action,
          heroClass: loadedSpot.hero_class,
          heroCards: loadedSpot.hero_cards,
        }
      : undefined;

  let form: React.ReactNode;
  if (!query) {
    form = <SolveForm onSubmitted={() => jobs.reload()} />;
  } else if (spot.error) {
    form = <ErrorState title="This spot cannot be solved" message={spot.error.message} />;
  } else if (!loadedSpot) {
    form = <LoadingState label="Loading your spot…" />;
  } else {
    form = (
      <div className="flex flex-col gap-4">
        <SpotBanner spot={loadedSpot} />
        <SolveForm
          initial={formFromSpec(loadedSpot.spec)}
          game={loadedSpot.spec.game}
          onSubmitted={(job) => {
            setSpotJobs((s) => new Set(s).add(job.id));
            setAwaiting(job.id);
            jobs.reload();
          }}
        />
      </div>
    );
  }

  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
      <Card>
        <CardHeader title="New postflop solve" subtitle="Heads-up; flop, turn or river root" />
        <CardBody>{form}</CardBody>
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
            <ResultView
              key={selected}
              jobId={selected}
              follow={follow}
              onLocked={(job) => {
                setAwaiting(job.id);
                jobs.reload();
              }}
            />
          </CardBody>
        </Card>
      )}
    </div>
  );
}

function FromQuery() {
  const p = useSearchParams();
  const game = p.get("game");
  const hand = p.get("hand");
  const decision = p.get("decision");
  return <Workbench query={game && hand && decision ? { game, hand, decision } : null} />;
}

export function SolverWorkbench() {
  return (
    <Suspense fallback={<LoadingState label="Loading…" />}>
      <FromQuery />
    </Suspense>
  );
}

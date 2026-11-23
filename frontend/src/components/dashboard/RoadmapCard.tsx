import { Circle, CircleCheck, CircleDashed, CircleDot } from "lucide-react";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";

const MILESTONES = [
  {
    n: "1",
    title: "Foundation",
    detail: "Engine, evaluator, ranges, equity, rake, betting rules, schema, app",
    state: "done",
  },
  {
    n: "2",
    title: "Solver validation",
    detail:
      "Independent best response; GTOpen checked on river, turn and push/fold, and under Miri. Full-range flop checks next",
    state: "current",
  },
  {
    n: "3",
    title: "Postflop solving",
    detail: "Heads-up solves with exploitability, node-by-node browsing, and solving spots from your own hands",
    state: "done",
  },
  {
    n: "4",
    title: "Job infrastructure",
    detail: "Local worker done; queue workers and result storage at scale",
    state: "next",
  },
  {
    n: "5",
    title: "Preflop",
    detail: "GTOpen's preflop model in the app, labelled approximate; heads-up push/fold validated",
    state: "current",
  },
  {
    n: "5a",
    title: "Academy",
    detail: "Lesson framework and five demonstration lessons; the curriculum is still to be written",
    state: "current",
  },
  {
    n: "6",
    title: "Trainer",
    detail: "Push/fold drills graded against the exact equilibrium; other drills wait for validated solutions",
    state: "current",
  },
  {
    n: "7",
    title: "Hand analysis & analytics",
    detail: "Analytics from your games and drills; hand-history import planned",
    state: "current",
  },
] as const;

export function RoadmapCard() {
  return (
    <Card>
      <CardHeader title="Roadmap" subtitle="Each milestone gates the next; see docs/roadmap.md" />
      <CardBody>
        <ol className="flex flex-col gap-3">
          {MILESTONES.map((m) => {
            const Icon =
              m.state === "done"
                ? CircleCheck
                : m.state === "current"
                  ? CircleDot
                  : m.state === "next"
                    ? CircleDashed
                    : Circle;
            return (
              <li key={m.n} className="flex gap-3">
                <Icon
                  aria-hidden
                  className={
                    m.state === "current" || m.state === "done"
                      ? "mt-0.5 size-4 text-accent"
                      : "mt-0.5 size-4 text-subtle"
                  }
                />
                <div className="min-w-0">
                  <p className="text-sm text-fg">
                    <span className="font-mono text-xs text-subtle">M{m.n}</span> {m.title}
                    {m.state === "done" && <span className="ml-2 text-xs text-muted">Done</span>}
                    {m.state === "current" && <span className="ml-2 text-xs text-accent">In progress</span>}
                    {m.state === "next" && <span className="ml-2 text-xs text-muted">Next</span>}
                  </p>
                  <p className="text-xs text-muted">{m.detail}</p>
                </div>
              </li>
            );
          })}
        </ol>
      </CardBody>
    </Card>
  );
}

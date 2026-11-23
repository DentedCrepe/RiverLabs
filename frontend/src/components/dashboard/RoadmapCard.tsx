import { Circle, CircleCheck, CircleDashed, CircleDot } from "lucide-react";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";

const MILESTONES = [
  {
    n: 1,
    title: "Foundation",
    detail: "Engine, evaluator, ranges, equity, rake, betting rules, schema, app",
    state: "done",
  },
  {
    n: 2,
    title: "Solver validation",
    detail: "Independent best response; GTOpen checked on river, turn and push/fold. Full-range flop checks next",
    state: "current",
  },
  {
    n: 3,
    title: "Postflop solving",
    detail: "Heads-up solves run in the app with exploitability. Node-by-node browsing next",
    state: "current",
  },
  {
    n: 4,
    title: "Job infrastructure",
    detail: "Local worker done; queue workers and result storage at scale",
    state: "next",
  },
  { n: 5, title: "Preflop", detail: "Six-max preflop with validated, clearly labelled approximations", state: "later" },
  { n: 6, title: "Trainer", detail: "Drills graded by EV loss against stored solutions", state: "later" },
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

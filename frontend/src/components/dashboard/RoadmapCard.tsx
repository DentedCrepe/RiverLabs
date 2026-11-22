import { Circle, CircleDashed, CircleDot } from "lucide-react";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";

const MILESTONES = [
  {
    n: 1,
    title: "Foundation",
    detail: "Engine core, evaluator, ranges, equity, rake, betting rules, schema, app shell",
    state: "current",
  },
  {
    n: 2,
    title: "Solver validation harness",
    detail: "CFR, CFR+, DCFR on Kuhn and Leduc; exploitability",
    state: "next",
  },
  { n: 3, title: "Postflop solving", detail: "Tree building, bet abstraction, river solves validated", state: "later" },
  { n: 4, title: "Job infrastructure", detail: "Workers, progress, cancellation, result storage", state: "later" },
  { n: 5, title: "Preflop", detail: "Solver-produced preflop strategies and visualisation", state: "later" },
  { n: 6, title: "Trainer", detail: "Drills graded by EV loss against stored solutions", state: "later" },
] as const;

export function RoadmapCard() {
  return (
    <Card>
      <CardHeader title="Roadmap" subtitle="Each milestone gates the next; see docs/roadmap.md" />
      <CardBody>
        <ol className="flex flex-col gap-3">
          {MILESTONES.map((m) => {
            const Icon = m.state === "current" ? CircleDot : m.state === "next" ? CircleDashed : Circle;
            return (
              <li key={m.n} className="flex gap-3">
                <Icon
                  aria-hidden
                  className={m.state === "current" ? "mt-0.5 size-4 text-accent" : "mt-0.5 size-4 text-subtle"}
                />
                <div className="min-w-0">
                  <p className="text-sm text-fg">
                    <span className="font-mono text-xs text-subtle">M{m.n}</span> {m.title}
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

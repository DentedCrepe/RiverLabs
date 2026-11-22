import type { Metadata } from "next";
import { SolverStatus } from "@/components/solver/SolverStatus";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Solver" };

const ALGORITHMS = [
  ["CFR", "Counterfactual regret minimisation (Zinkevich et al., 2007). Baseline for validation."],
  ["CFR+", "Regret floor at zero, alternating updates, linear averaging (Tammelin, 2014)."],
  ["Discounted CFR", "Discounts regrets and averages; defaults α = 1.5, β = 0, γ = 2 (Brown & Sandholm, 2019)."],
  [
    "Monte Carlo CFR",
    "Sampled traversal (external / outcome sampling) for very large trees; seeded for reproducibility.",
  ],
] as const;

export default function SolverPage() {
  return (
    <>
      <PageHeader
        title="Solver"
        status="interface_only"
        description="The solver contract, configuration model and job lifecycle are defined. No solving algorithm is implemented, so no solution can be produced yet."
      />
      <div className="grid gap-6 lg:grid-cols-2">
        <SolverStatus />
        <Card>
          <CardHeader title="Planned algorithms" subtitle="All share one Solver interface in engine/solver-interface" />
          <CardBody>
            <dl className="flex flex-col gap-3">
              {ALGORITHMS.map(([name, text]) => (
                <div key={name}>
                  <dt className="text-sm font-medium text-fg">{name}</dt>
                  <dd className="text-sm text-muted">{text}</dd>
                </div>
              ))}
            </dl>
          </CardBody>
        </Card>
        <Card className="lg:col-span-2">
          <CardHeader title="Validation before trust" />
          <CardBody className="text-sm text-muted">
            <ol className="list-decimal space-y-1.5 pl-5">
              <li>Implement CFR variants against small games with known equilibria (Kuhn, Leduc).</li>
              <li>Measure exploitability with an independent best-response computation; require convergence.</li>
              <li>
                Only then build hold&apos;em trees, starting with river spots checked by brute-force best response.
              </li>
              <li>Every stored result records engine version, solver version, configuration and exploitability.</li>
            </ol>
          </CardBody>
        </Card>
      </div>
    </>
  );
}

import type { Metadata } from "next";
import { SolverStatus } from "@/components/solver/SolverStatus";
import { SolverWorkbench } from "@/components/solver/SolverWorkbench";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Solver" };

export default function SolverPage() {
  return (
    <>
      <PageHeader
        title="Solver"
        status="partial"
        description="Heads-up postflop solving with GTOpen (Discounted CFR), pinned to an audited commit. Every result reports its exploitability; results are re-checked by RiverLab's own best-response calculation whenever it fits in memory. Preflop and multiway solving are not available here yet."
      />
      <div className="grid gap-6">
        <div className="grid gap-6 lg:grid-cols-2">
          <SolverStatus />
          <Card>
            <CardHeader title="How to read results" />
            <CardBody className="text-sm text-muted">
              <ul className="list-disc space-y-1.5 pl-5">
                <li>A solve is an equilibrium for the bet sizes you allow, not for every possible bet.</li>
                <li>
                  Exploitability is the most a perfect opponent could win against the strategy (% of the starting pot).
                </li>
                <li>Values are per hand, net of each player&apos;s half of the starting pot.</li>
                <li>Any approximation (e.g. rake rounding) is listed with the result.</li>
              </ul>
            </CardBody>
          </Card>
        </div>
        <SolverWorkbench />
      </div>
    </>
  );
}

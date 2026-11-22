import type { Metadata } from "next";
import { EquityTool } from "@/components/poker/EquityTool";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/States";
import { StatusBadge } from "@/components/ui/StatusBadge";

export const metadata: Metadata = { title: "Postflop" };

export default function PostflopPage() {
  return (
    <>
      <PageHeader
        title="Postflop"
        status="partial"
        description="All-in equity for hands and weighted ranges. Equity assumes the hand is played to showdown with no further betting — it is not a strategy."
      />
      <div className="grid gap-6">
        <Card>
          <CardHeader title="Equity calculator" subtitle="Exact enumeration or seeded Monte Carlo, with card removal" />
          <CardBody>
            <EquityTool />
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Postflop strategy explorer" actions={<StatusBadge status="planned" />} />
          <CardBody>
            <EmptyState
              title="No postflop solutions available"
              description="Strategy trees, EVs and node-by-node frequencies require the postflop solver (Milestone 3)."
            />
          </CardBody>
        </Card>
      </div>
    </>
  );
}

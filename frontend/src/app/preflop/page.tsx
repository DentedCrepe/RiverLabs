import type { Metadata } from "next";
import { RangeEditor } from "@/components/poker/RangeEditor";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/States";
import { StatusBadge } from "@/components/ui/StatusBadge";

export const metadata: Metadata = { title: "Preflop" };

export default function PreflopPage() {
  return (
    <>
      <PageHeader
        title="Preflop"
        status="partial"
        description="Build and inspect ranges with exact combo counts and card removal. Preflop strategies will only be shown once they are produced by a validated solver."
      />
      <RangeEditor />
      <Card className="mt-6">
        <CardHeader title="Six-max preflop strategies" actions={<StatusBadge status="planned" />} />
        <CardBody>
          <EmptyState
            title="No preflop solutions available"
            description="RiverLab does not display illustrative or third-party strategy charts as GTO. Strategies arrive with Milestone 5."
          />
        </CardBody>
      </Card>
    </>
  );
}

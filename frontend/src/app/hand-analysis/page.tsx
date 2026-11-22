import type { Metadata } from "next";
import { ShowdownTool } from "@/components/poker/ShowdownTool";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/States";
import { StatusBadge } from "@/components/ui/StatusBadge";

export const metadata: Metadata = { title: "Hand Analysis" };

export default function HandAnalysisPage() {
  return (
    <>
      <PageHeader
        title="Hand Analysis"
        status="partial"
        description="Showdown evaluation is available now. Hand-history import and street-by-street review are planned."
      />
      <div className="grid gap-6 lg:grid-cols-[3fr_2fr]">
        <Card>
          <CardHeader title="Showdown evaluator" subtitle="Correct kickers, split pots, board-playing hands" />
          <CardBody>
            <ShowdownTool />
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Hand history import" actions={<StatusBadge status="planned" />} />
          <CardBody>
            <EmptyState
              title="No hand histories"
              description="Importing requires a parser validated against real site histories (Milestone 7)."
            />
          </CardBody>
        </Card>
      </div>
    </>
  );
}

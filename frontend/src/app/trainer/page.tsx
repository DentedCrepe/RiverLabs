import type { Metadata } from "next";
import { Card, CardBody } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/States";

export const metadata: Metadata = { title: "Trainer" };

export default function Page() {
  return (
    <>
      <PageHeader
        title="Trainer"
        status="planned"
        description="Practice decisions in six-max spots and get graded by EV loss."
      />
      <Card>
        <CardBody>
          <EmptyState
            title="Not available yet"
            description="Grading needs stored, validated solver results. The trainer will be built after the solver (Milestone 6) so no drill is graded against invented answers."
          />
        </CardBody>
      </Card>
    </>
  );
}

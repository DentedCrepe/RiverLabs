import type { Metadata } from "next";
import { Card, CardBody } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/States";

export const metadata: Metadata = { title: "Solution Library" };

export default function Page() {
  return (
    <>
      <PageHeader
        title="Solution Library"
        status="planned"
        description="Saved solver results with their full configuration, engine version and exploitability."
      />
      <Card>
        <CardBody>
          <EmptyState
            title="No solutions"
            description="The library is empty because no solver has produced a result yet."
          />
        </CardBody>
      </Card>
    </>
  );
}

import type { Metadata } from "next";
import { Card, CardBody } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/States";

export const metadata: Metadata = { title: "Analytics" };

export default function Page() {
  return (
    <>
      <PageHeader
        title="Analytics"
        status="planned"
        description="Performance analytics computed from your own hands and training decisions."
      />
      <Card>
        <CardBody>
          <EmptyState
            title="No data"
            description="Analytics will only summarise real imported hands and graded decisions (Milestone 7). No sample statistics are shown."
          />
        </CardBody>
      </Card>
    </>
  );
}

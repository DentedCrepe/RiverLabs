import type { Metadata } from "next";
import { Library } from "@/components/library/Library";
import { Card, CardBody } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Solution Library" };

export default function Page() {
  return (
    <>
      <PageHeader
        title="Solution Library"
        status="available"
        description="Every saved solver result with its full configuration, engine and solver versions, and its exploitability (postflop) or model gap (preflop, an approximate model unless marked validated)."
      />
      <Card>
        <CardBody>
          <Library />
        </CardBody>
      </Card>
    </>
  );
}

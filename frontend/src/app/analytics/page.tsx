import type { Metadata } from "next";
import { Analytics } from "@/components/analytics/Analytics";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Analytics" };

export default function Page() {
  return (
    <>
      <PageHeader
        title="Analytics"
        status="partial"
        description="Computed only from your own data: every hand you played against the bots, and every graded trainer answer. Imported hand histories are planned."
      />
      <Analytics />
    </>
  );
}

import type { Metadata } from "next";
import { RevisionView } from "@/components/academy/RevisionView";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Academy revision" };

export default function RevisionPage() {
  return (
    <>
      <PageHeader
        title="Revision"
        status="partial"
        description="Questions due again, starting with the ones you got wrong."
      />
      <RevisionView />
    </>
  );
}

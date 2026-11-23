import type { Metadata } from "next";
import { AcademyDashboard } from "@/components/academy/AcademyDashboard";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Academy" };

export default function AcademyPage() {
  return (
    <>
      <PageHeader
        title="Academy"
        status="partial"
        description="Learn no-limit hold'em and game theory from the basics to solver theory. Every lesson labels what is an established fact, a heuristic, an approximation or a solver-specific result; numbers in lessons and questions are computed by RiverLab's engine. The curriculum is being written: planned topics are shown greyed out."
      />
      <AcademyDashboard />
    </>
  );
}

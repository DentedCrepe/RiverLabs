import type { Metadata } from "next";
import { TrainerTabs } from "@/components/trainer/TrainerTabs";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Trainer" };

export default function Page() {
  return (
    <>
      <PageHeader
        title="Trainer"
        status="partial"
        description="Drills graded by EV: heads-up push/fold against the exact equilibrium, and postflop decisions dealt from your own saved solves, each shown with its exploitability. Nothing is graded against invented answers."
      />
      <TrainerTabs />
    </>
  );
}

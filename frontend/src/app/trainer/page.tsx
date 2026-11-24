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
        description="Heads-up push/fold graded by EV against the exact equilibrium; postflop decisions from your saved solves, graded by EV with their exploitability; and preflop decisions from your saved preflop solves, compared with GTOpen's preflop model (an approximate model). Nothing is graded against invented answers."
      />
      <TrainerTabs />
    </>
  );
}

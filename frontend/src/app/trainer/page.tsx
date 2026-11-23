import type { Metadata } from "next";
import { PushFoldTrainer } from "@/components/trainer/PushFoldTrainer";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Trainer" };

export default function Page() {
  return (
    <>
      <PageHeader
        title="Trainer"
        status="partial"
        description="Heads-up push/fold drills graded by EV against the exact equilibrium. Drills for other spots will follow once their solutions are validated, so nothing is graded against invented answers."
      />
      <PushFoldTrainer />
    </>
  );
}

import type { Metadata } from "next";
import { PlayWorkbench } from "@/components/play/PlayWorkbench";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Play" };

export default function PlayPage() {
  return (
    <>
      <PageHeader
        title="Play"
        status="approximate"
        description="Play cash, fast-fold or sit & go games against bots at a table you configure, then review your statistics and the weaknesses found. Bots use GTOpen's preflop model (approximate) and simple postflop rules; every finding states what it is measured against."
      />
      <PlayWorkbench />
    </>
  );
}

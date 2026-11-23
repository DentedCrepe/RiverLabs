import { Spade } from "lucide-react";
import type { Metadata } from "next";
import { CapabilitiesCard } from "@/components/dashboard/CapabilitiesCard";
import { DailyGoalsCard } from "@/components/dashboard/DailyGoalsCard";
import { DefaultConfigCard } from "@/components/dashboard/DefaultConfigCard";
import { PerformanceCard, TrainingProgressCard } from "@/components/dashboard/ProgressCards";
import { RecentSolves } from "@/components/dashboard/RecentSolves";
import { RoadmapCard } from "@/components/dashboard/RoadmapCard";
import { SystemStatusCard } from "@/components/dashboard/SystemStatusCard";
import { ShowdownTool } from "@/components/poker/ShowdownTool";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Dashboard" };

export default function DashboardPage() {
  return (
    <>
      <PageHeader
        title="Dashboard"
        description="Six-max no-limit hold'em research workspace. Every number on this page comes from the validated engine or the live API; unfinished areas are labelled."
      />
      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <DailyGoalsCard />
        </div>
        <SystemStatusCard />
        <DefaultConfigCard />
        <RoadmapCard />
        <div className="lg:col-span-2">
          <CapabilitiesCard />
        </div>
        <Card>
          <CardHeader title="Showdown" subtitle="7-card evaluator" icon={<Spade className="size-4" />} />
          <CardBody>
            <ShowdownTool compact />
          </CardBody>
        </Card>
        <RecentSolves />
        <TrainingProgressCard />
        <PerformanceCard />
      </div>
    </>
  );
}

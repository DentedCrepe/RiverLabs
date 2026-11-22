import { BarChart3, Cpu, GraduationCap, Spade } from "lucide-react";
import type { Metadata } from "next";
import { CapabilitiesCard } from "@/components/dashboard/CapabilitiesCard";
import { DefaultConfigCard } from "@/components/dashboard/DefaultConfigCard";
import { RoadmapCard } from "@/components/dashboard/RoadmapCard";
import { SystemStatusCard } from "@/components/dashboard/SystemStatusCard";
import { ShowdownTool } from "@/components/poker/ShowdownTool";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/States";
import { StatusBadge } from "@/components/ui/StatusBadge";

export const metadata: Metadata = { title: "Dashboard" };

const PLANNED = [
  {
    title: "Recent solves",
    icon: <Cpu aria-hidden className="size-6" />,
    text: "Solver jobs and their exploitability will appear here once the solver exists and has been validated (Milestones 2–4).",
  },
  {
    title: "Training progress",
    icon: <GraduationCap aria-hidden className="size-6" />,
    text: "Drill results graded by EV loss against stored solutions (Milestone 6).",
  },
  {
    title: "Performance",
    icon: <BarChart3 aria-hidden className="size-6" />,
    text: "Metrics computed from your own imported hands. Nothing is shown until real data exists.",
  },
];

export default function DashboardPage() {
  return (
    <>
      <PageHeader
        title="Dashboard"
        description="Six-max no-limit hold'em research workspace. Every number on this page comes from the validated engine or the live API; unfinished areas are labelled."
      />
      <div className="grid gap-6 lg:grid-cols-3">
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
        {PLANNED.map((p) => (
          <Card key={p.title}>
            <CardHeader title={p.title} actions={<StatusBadge status="planned" />} />
            <CardBody>
              <EmptyState title="No data yet" description={p.text} icon={p.icon} />
            </CardBody>
          </Card>
        ))}
      </div>
    </>
  );
}

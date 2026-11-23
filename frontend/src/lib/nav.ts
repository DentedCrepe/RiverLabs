import type { LucideIcon } from "lucide-react";
import {
  BarChart3,
  BookOpen,
  Cpu,
  GraduationCap,
  Grid3x3,
  History,
  LayoutDashboard,
  Layers,
  Settings,
  Swords,
  Target,
} from "lucide-react";

/** Implementation status shown next to navigation items and on pages. */
export type FeatureStatus = "available" | "approximate" | "partial" | "interface_only" | "planned";

export interface NavItem {
  href: string;
  label: string;
  icon: LucideIcon;
  status: FeatureStatus;
  description: string;
}

export const NAV_ITEMS: readonly NavItem[] = [
  {
    href: "/",
    label: "Dashboard",
    icon: LayoutDashboard,
    status: "available",
    description: "System status, capabilities and roadmap",
  },
  {
    href: "/preflop",
    label: "Preflop",
    icon: Grid3x3,
    status: "approximate",
    description: "Range editor and GTOpen's preflop model (approximate; push/fold validated)",
  },
  {
    href: "/postflop",
    label: "Postflop",
    icon: Layers,
    status: "partial",
    description: "Equity calculator; postflop strategies are planned",
  },
  {
    href: "/solver",
    label: "Solver",
    icon: Cpu,
    status: "partial",
    description: "Heads-up postflop solving (GTOpen); multiway planned",
  },
  {
    href: "/academy",
    label: "Academy",
    icon: GraduationCap,
    status: "partial",
    description: "Poker and game theory lessons with interactive tools and knowledge checks",
  },
  {
    href: "/play",
    label: "Play",
    icon: Swords,
    status: "approximate",
    description: "Games against bots with a session report (bots use an approximate preflop model)",
  },
  {
    href: "/trainer",
    label: "Trainer",
    icon: Target,
    status: "partial",
    description: "Heads-up push/fold drills graded against the exact equilibrium; more drills planned",
  },
  {
    href: "/hand-analysis",
    label: "Hand Analysis",
    icon: History,
    status: "partial",
    description: "Showdown evaluator; history import is planned",
  },
  {
    href: "/analytics",
    label: "Analytics",
    icon: BarChart3,
    status: "planned",
    description: "Performance metrics from your own hands",
  },
  {
    href: "/library",
    label: "Solution Library",
    icon: BookOpen,
    status: "available",
    description: "Saved solver results",
  },
  {
    href: "/settings",
    label: "Settings",
    icon: Settings,
    status: "available",
    description: "Game configuration: stacks and rake",
  },
];

export function isActive(pathname: string, href: string): boolean {
  if (href === "/") return pathname === "/";
  return pathname === href || pathname.startsWith(`${href}/`);
}

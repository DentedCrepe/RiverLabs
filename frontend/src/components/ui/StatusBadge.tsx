import type { FeatureStatus } from "@/lib/nav";
import { cn } from "./cn";

const STYLES: Record<FeatureStatus, { label: string; className: string }> = {
  available: { label: "Available", className: "bg-accent-dim text-accent-strong border-accent/30" },
  approximate: { label: "Approximate model", className: "bg-warn-dim text-warn border-warn/30" },
  partial: { label: "In development", className: "bg-info-dim text-info border-info/30" },
  interface_only: { label: "Interface only", className: "bg-warn-dim text-warn border-warn/30" },
  planned: { label: "Planned", className: "bg-surface-3 text-muted border-line-strong" },
};

export function statusLabel(status: FeatureStatus): string {
  return STYLES[status].label;
}

export function StatusBadge({ status, className }: { status: FeatureStatus; className?: string }) {
  const s = STYLES[status];
  return (
    <span
      className={cn(
        "inline-flex items-center rounded border px-1.5 py-px text-[10px] font-medium tracking-wide uppercase",
        s.className,
        className,
      )}
    >
      {s.label}
    </span>
  );
}

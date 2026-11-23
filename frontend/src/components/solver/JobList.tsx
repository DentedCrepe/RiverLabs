"use client";

import { CircleSlash, Eye, Trash2 } from "lucide-react";
import { ApiError, api } from "@/lib/api";
import type { SolverJob } from "@/lib/types";
import { Button } from "@/components/ui/Button";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

const STATUS: Record<SolverJob["status"], string> = {
  queued: "bg-surface-3 text-muted",
  running: "bg-info-dim text-info",
  succeeded: "bg-accent-dim text-accent-strong",
  failed: "bg-danger-dim text-danger",
  cancelled: "bg-warn-dim text-warn",
};

export function JobList({
  jobs,
  error,
  loading,
  selected,
  onSelect,
  onChanged,
}: {
  jobs: SolverJob[] | null;
  error: ApiError | null;
  loading: boolean;
  selected: string | null;
  onSelect: (id: string) => void;
  onChanged: () => void;
}) {
  if (error) return <ErrorState message={error.message} />;
  if (loading && !jobs) return <LoadingState />;
  if (!jobs || jobs.length === 0)
    return <EmptyState title="No solves yet" description="Queued and finished solves appear here." />;

  async function cancel(id: string) {
    try {
      await api.post(`/api/v1/solver/jobs/${id}/cancel`, {});
    } finally {
      onChanged();
    }
  }

  async function remove(id: string) {
    try {
      await api.delete(`/api/v1/solver/jobs/${id}`);
    } finally {
      onChanged();
    }
  }

  return (
    <ul className="divide-y divide-line">
      {jobs.map((j) => {
        const active = j.status === "queued" || j.status === "running";
        const pct = j.progress.exploitability_pct_pot;
        return (
          <li
            key={j.id}
            className={cn("flex flex-wrap items-center gap-3 py-2.5", selected === j.id && "bg-surface-2/60")}
          >
            <span
              className={cn("rounded px-1.5 py-px text-[10px] font-medium tracking-wide uppercase", STATUS[j.status])}
            >
              {j.status}
            </span>
            <span className="font-mono text-xs text-muted tabular">{new Date(j.created_at).toLocaleTimeString()}</span>
            <span className="font-mono text-xs text-fg tabular">
              {j.iterations.toLocaleString()} it
              {pct !== undefined && ` · ${pct.toFixed(3)}% pot`}
            </span>
            {j.cancel_requested && j.status === "running" && <span className="text-xs text-warn">cancelling…</span>}
            {j.error_message && <span className="w-full text-xs text-danger">{j.error_message}</span>}
            <span className="ml-auto flex gap-1">
              {j.status === "succeeded" && (
                <Button size="sm" variant="ghost" onClick={() => onSelect(j.id)} icon={<Eye className="size-3.5" />}>
                  View
                </Button>
              )}
              {!active && (
                <Button
                  size="sm"
                  variant="ghost"
                  aria-label="Delete solve"
                  onClick={() => remove(j.id)}
                  icon={<Trash2 className="size-3.5" />}
                />
              )}
              {active && !j.cancel_requested && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => cancel(j.id)}
                  icon={<CircleSlash className="size-3.5" />}
                >
                  Cancel
                </Button>
              )}
            </span>
          </li>
        );
      })}
    </ul>
  );
}

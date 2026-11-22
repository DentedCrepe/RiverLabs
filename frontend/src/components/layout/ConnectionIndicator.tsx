"use client";

import { useApi } from "@/lib/useApi";
import type { Readiness } from "@/lib/types";
import { cn } from "@/components/ui/cn";

/** Live backend/database status, polled through the same-origin proxy. */
export function ConnectionIndicator() {
  const { data, error, loading } = useApi<Readiness>("/health/ready", { refreshMs: 30_000 });
  const state = loading && !data ? "checking" : error ? "offline" : data?.status === "ok" ? "online" : "degraded";
  const text = {
    checking: "Checking API…",
    offline: "API unreachable",
    degraded: "Database unavailable",
    online: `Engine ${data?.engine ?? ""}`,
  }[state];
  return (
    <span
      role="status"
      className="inline-flex items-center gap-2 rounded-md border border-line px-2.5 py-1 text-xs text-muted"
    >
      <span
        aria-hidden
        className={cn(
          "size-2 rounded-full",
          state === "online" && "bg-accent",
          state === "degraded" && "bg-warn",
          state === "offline" && "bg-danger",
          state === "checking" && "animate-pulse bg-subtle",
        )}
      />
      <span className="font-mono tabular">{text}</span>
    </span>
  );
}

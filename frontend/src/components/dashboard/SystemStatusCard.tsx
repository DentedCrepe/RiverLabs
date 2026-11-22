"use client";

import { Activity, Database, Server } from "lucide-react";
import type { ReactNode } from "react";
import type { Meta, Readiness } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Skeleton } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

function Row({ icon, label, value, ok }: { icon: ReactNode; label: string; value: ReactNode; ok: boolean | null }) {
  return (
    <li className="flex items-center gap-3 py-2">
      <span className="text-subtle">{icon}</span>
      <span className="flex-1 text-sm text-muted">{label}</span>
      <span className="font-mono text-sm text-fg tabular">{value}</span>
      <span
        aria-hidden
        className={cn("size-2 rounded-full", ok === null ? "bg-subtle" : ok ? "bg-accent" : "bg-danger")}
      />
    </li>
  );
}

export function SystemStatusCard() {
  const ready = useApi<Readiness>("/health/ready", { refreshMs: 15_000 });
  const meta = useApi<Meta>("/api/v1/meta");
  const loading = ready.loading && !ready.data;
  return (
    <Card>
      <CardHeader
        title="System status"
        subtitle="Live, polled every 15 s"
        icon={<Activity className="size-4" />}
        actions={
          <Button size="sm" variant="ghost" onClick={ready.reload}>
            Refresh
          </Button>
        }
      />
      <CardBody className="py-1">
        {loading ? (
          <div className="flex flex-col gap-3 py-3">
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-4/5" />
            <Skeleton className="h-4 w-3/5" />
          </div>
        ) : (
          <ul className="divide-y divide-line">
            <Row
              icon={<Server className="size-4" />}
              label="API"
              value={ready.error ? "unreachable" : `v${meta.data?.api_version ?? "…"}`}
              ok={!ready.error}
            />
            <Row
              icon={<Database className="size-4" />}
              label="PostgreSQL"
              value={ready.data?.database ?? "unknown"}
              ok={ready.data ? ready.data.database === "ok" : ready.error ? false : null}
            />
            <Row
              icon={<Activity className="size-4" />}
              label="Rust engine"
              value={ready.data ? `v${ready.data.engine}` : "unknown"}
              ok={ready.data ? true : ready.error ? false : null}
            />
          </ul>
        )}
        {ready.error && <p className="pb-3 text-xs text-danger">{ready.error.message}</p>}
      </CardBody>
    </Card>
  );
}

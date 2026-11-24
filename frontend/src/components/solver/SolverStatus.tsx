"use client";

import { Cpu } from "lucide-react";
import type { SolverCapabilities } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { StatusBadge } from "@/components/ui/StatusBadge";

export function SolverStatus() {
  const { data, error, loading } = useApi<SolverCapabilities>("/api/v1/solver/capabilities");
  return (
    <Card>
      <CardHeader title="Solver backend" icon={<Cpu className="size-4" />} />
      <CardBody className="flex flex-col gap-3">
        {loading && !data ? (
          <LoadingState />
        ) : error ? (
          <ErrorState message={error.message} />
        ) : data ? (
          <>
            <div className="flex items-center gap-2">
              <StatusBadge status={data.available_backends.length ? "partial" : "interface_only"} />
              <span className="text-sm text-fg">
                {data.engines.length
                  ? data.engines
                      .map(
                        (e) =>
                          `${e.name} @ ${e.commit.slice(0, 10)}${e.patches?.length ? ` + ${e.patches.join(", ")}` : ""}`,
                      )
                      .join(", ")
                  : "No backend enabled"}
              </span>
            </div>
            <p className="text-sm text-muted">{data.message}</p>
            {data.engines.map((e) => (
              <p key={e.name} className="text-xs text-subtle">
                Scope: {e.scope}. Source: {e.repository}
              </p>
            ))}
          </>
        ) : null}
      </CardBody>
    </Card>
  );
}

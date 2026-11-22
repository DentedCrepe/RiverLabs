"use client";

import { Cpu } from "lucide-react";
import type { SolverCapabilities } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { StatusBadge } from "@/components/ui/StatusBadge";

export function SolverStatus() {
  const { data, error, loading } = useApi<SolverCapabilities>("/api/v1/solver/capabilities");
  return (
    <Card>
      <CardHeader title="Solver backends" icon={<Cpu className="size-4" />} />
      <CardBody className="flex flex-col gap-3">
        {loading && !data ? (
          <LoadingState />
        ) : error ? (
          <ErrorState message={error.message} />
        ) : data ? (
          <>
            <div className="flex items-center gap-2">
              <StatusBadge status={data.available_backends.length ? "available" : "interface_only"} />
              <span className="text-sm text-fg">
                {data.available_backends.length ? data.available_backends.join(", ") : "No backend registered"}
              </span>
            </div>
            <p className="text-sm text-muted">{data.message}</p>
            <div>
              <Button disabled title="No solver backend is available">
                New solve
              </Button>
            </div>
          </>
        ) : null}
      </CardBody>
    </Card>
  );
}

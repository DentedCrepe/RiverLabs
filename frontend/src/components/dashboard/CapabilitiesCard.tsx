"use client";

import { FlaskConical } from "lucide-react";
import type { Meta } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { ErrorState, Skeleton } from "@/components/ui/States";
import { StatusBadge } from "@/components/ui/StatusBadge";

/** What the platform can and cannot do today, as reported by the backend. */
export function CapabilitiesCard() {
  const { data, error, loading } = useApi<Meta>("/api/v1/meta");
  return (
    <Card>
      <CardHeader
        title="Engine capabilities"
        subtitle="Reported by the API — nothing here is inferred by the UI"
        icon={<FlaskConical className="size-4" />}
      />
      <CardBody>
        {error ? (
          <ErrorState title="Capabilities unavailable" message={error.message} />
        ) : loading || !data ? (
          <div className="grid gap-3 sm:grid-cols-2">
            {Array.from({ length: 6 }, (_, i) => (
              <Skeleton key={i} className="h-14" />
            ))}
          </div>
        ) : (
          <ul className="grid gap-2 sm:grid-cols-2">
            {data.capabilities.map((c) => (
              <li key={c.key} className="rounded-md border border-line bg-surface-2/60 px-3 py-2.5">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium text-fg">{c.name}</span>
                  <StatusBadge status={c.status} />
                </div>
                <p className="mt-1 text-xs text-muted">{c.detail}</p>
              </li>
            ))}
          </ul>
        )}
      </CardBody>
    </Card>
  );
}

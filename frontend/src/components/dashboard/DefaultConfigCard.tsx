"use client";

import { Settings } from "lucide-react";
import Link from "next/link";
import type { GameConfig } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBb, formatBps } from "@/lib/units";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { ErrorState, Skeleton } from "@/components/ui/States";

export function DefaultConfigCard() {
  const { data, error } = useApi<GameConfig>("/api/v1/game-configs/default");
  const rows: [string, string][] = data
    ? [
        ["Game", `NLHE · ${data.players}-max`],
        ["Blinds", `${formatBb(data.small_blind)} / 1 bb`],
        ["Effective stack", `${formatBb(data.stack.effective_stack)} bb`],
        ["Rake", `${formatBps(data.rake.percentage_bps)}%`],
        [
          "Rake cap",
          data.rake.cap
            ? data.rake.cap.unit === "big_blinds"
              ? `${formatBb(data.rake.cap.mbb)} bb`
              : `${data.rake.cap.minor_units} minor units`
            : "None",
        ],
        ["No flop, no drop", data.rake.no_flop_no_drop ? "Yes" : "No"],
      ]
    : [];
  return (
    <Card>
      <CardHeader
        title="Default game"
        subtitle="Engine default configuration"
        icon={<Settings className="size-4" />}
        actions={
          <Link href="/settings" className="text-xs text-accent hover:text-accent-strong">
            Configure
          </Link>
        }
      />
      <CardBody>
        {error ? (
          <ErrorState message={error.message} />
        ) : !data ? (
          <div className="flex flex-col gap-2.5">
            {Array.from({ length: 6 }, (_, i) => (
              <Skeleton key={i} className="h-4" />
            ))}
          </div>
        ) : (
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
            {rows.map(([k, v]) => (
              <div key={k} className="contents">
                <dt className="text-muted">{k}</dt>
                <dd className="text-right font-mono text-fg tabular">{v}</dd>
              </div>
            ))}
          </dl>
        )}
      </CardBody>
    </Card>
  );
}

"use client";

import { useState } from "react";
import type { HandRecord } from "@/lib/play";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { HandRows } from "./HandHistory";

type Filter = "all" | "mistake" | "rare";
type Page = {
  total: number;
  hands: { session_id: string; format: string; session_created_at: string; record: HandRecord }[];
};

const PAGE = 30;
const FILTERS: [Filter, string][] = [
  ["all", "All hands"],
  ["mistake", "Flagged"],
  ["rare", "Unusual"],
];

/** Your hands from every game against the bots, newest first. */
export function YourHands() {
  const [filter, setFilter] = useState<Filter>("all");
  const [offset, setOffset] = useState(0);
  const { data, error, loading } = useApi<Page>(
    `/api/v1/play/hands?limit=${PAGE}&offset=${offset}${filter === "all" ? "" : `&verdict=${filter}`}`,
  );
  return (
    <div className="flex flex-col gap-3">
      <div role="tablist" aria-label="Which hands" className="flex flex-wrap gap-1">
        {FILTERS.map(([k, label]) => (
          <button
            key={k}
            role="tab"
            type="button"
            aria-selected={filter === k}
            onClick={() => {
              setFilter(k);
              setOffset(0);
            }}
            className={cn(
              "rounded-md px-3 py-1 text-sm",
              filter === k ? "bg-surface-3 text-fg" : "text-muted hover:bg-surface-2",
            )}
          >
            {label}
          </button>
        ))}
      </div>
      {error ? (
        <ErrorState message={error.message} />
      ) : loading && !data ? (
        <LoadingState label="Loading your hands…" />
      ) : !data || data.total === 0 ? (
        <EmptyState
          title={filter === "all" ? "No hands yet" : "No hands match"}
          description={
            filter === "all"
              ? "Hands you play on the Play page appear here."
              : "None of your decisions got this verdict from the instant feedback."
          }
        />
      ) : (
        <>
          <p className="text-xs text-muted">
            {data.total} hand{data.total === 1 ? "" : "s"}. Tap one to replay it; heads-up decisions after the flop can
            be solved.
          </p>
          <HandRows
            ariaLabel="Your hands"
            items={data.hands.map((h) => ({
              sessionId: h.session_id,
              hand: h.record,
              label: new Date(h.session_created_at).toLocaleDateString(),
            }))}
          />
          {data.total > PAGE && (
            <div className="flex items-center gap-2 text-xs text-muted">
              <Button size="sm" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE))}>
                Newer
              </Button>
              <span className="font-mono tabular">
                {offset + 1}–{Math.min(offset + PAGE, data.total)} of {data.total}
              </span>
              <Button size="sm" disabled={offset + PAGE >= data.total} onClick={() => setOffset(offset + PAGE)}>
                Older
              </Button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

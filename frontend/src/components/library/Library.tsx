"use client";

import { ChevronRight, CircleCheck, TriangleAlert } from "lucide-react";
import { useState } from "react";
import { formatBb } from "@/lib/units";
import { useApi } from "@/lib/useApi";
import { CardRow } from "@/components/poker/CardText";
import { PreflopResultView } from "@/components/preflop/PreflopResultView";
import { ResultView } from "@/components/solver/ResultView";
import { WholeHand } from "./WholeHand";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

/** One saved result (backend LibraryEntry). */
export type LibraryEntry = {
  job_id: string;
  kind: "postflop" | "preflop";
  finished_at: string | null;
  engine_version: string;
  solver_backend: string;
  solver_version: string;
  iterations: number;
  stop_reason: string | null;
  tree_available: boolean;
  board: string[] | null;
  ranges: string[] | null;
  starting_pot_mbb: number | null;
  effective_stack_mbb: number | null;
  exploitability_mbb: number | null;
  exploitability_pct_pot: number | null;
  independent_check: boolean | null;
  approximations: number;
  players: number | null;
  positions: string[] | null;
  validated_scope: string | null;
  model_gap_mbb: number | null;
  /** Set when the solve is one street of a solver review of a hand you played. */
  hand?: { session_id: string; hand_no: number; street: string } | null;
};

type Row = { kind: "solve"; e: LibraryEntry } | { kind: "hand"; key: string; entries: LibraryEntry[] };

/** Street solves of the same hand become one row; other solves stay as they are. */
export function libraryRows(data: LibraryEntry[]): Row[] {
  const rows: Row[] = [];
  const hands = new Map<string, LibraryEntry[]>();
  for (const e of data) {
    if (!e.hand) {
      rows.push({ kind: "solve", e });
      continue;
    }
    const key = `${e.hand.session_id}:${e.hand.hand_no}`;
    const group = hands.get(key);
    if (group) group.push(e);
    else {
      const entries = [e];
      hands.set(key, entries);
      rows.push({ kind: "hand", key, entries });
    }
  }
  return rows;
}

const ORDER = ["flop", "turn", "river"];

function HandRow({ entries }: { entries: LibraryEntry[] }) {
  const sorted = [...entries].sort((a, b) => ORDER.indexOf(a.hand!.street) - ORDER.indexOf(b.hand!.street));
  const last = sorted[sorted.length - 1];
  const worst = Math.max(...sorted.map((e) => e.exploitability_pct_pot ?? 0));
  return (
    <span className="flex flex-wrap items-center gap-2">
      <span className="font-medium text-fg">Hand #{last.hand!.hand_no}</span>
      {last.board && <CardRow cards={last.board} size="sm" />}
      <span className="text-muted">
        {sorted.map((e) => e.hand!.street).join(", ")} solved · up to {worst.toFixed(3)}% pot exploitable
      </span>
    </span>
  );
}

type Kind = "all" | "postflop" | "preflop";

function Quality({ e }: { e: LibraryEntry }) {
  if (e.kind === "postflop") {
    return (
      <span className="flex flex-wrap items-center gap-1.5">
        <span className="font-mono tabular">
          {e.exploitability_pct_pot !== null ? `${e.exploitability_pct_pot.toFixed(3)}% pot` : "—"}
        </span>
        <span className="text-subtle">exploitable</span>
        {e.independent_check && (
          <span className="inline-flex items-center gap-0.5 text-accent-strong" title="Re-checked by RiverLab">
            <CircleCheck aria-hidden className="size-3" /> re-checked
          </span>
        )}
      </span>
    );
  }
  return e.validated_scope === "heads_up_push_fold" ? (
    <span className="inline-flex items-center gap-1 text-accent-strong">
      <CircleCheck aria-hidden className="size-3" /> validated push/fold
    </span>
  ) : (
    <span className="inline-flex items-center gap-1 text-warn">
      <TriangleAlert aria-hidden className="size-3" /> approximate model
      {e.model_gap_mbb !== null && (
        <span className="font-mono text-muted tabular">· gap {e.model_gap_mbb.toFixed(2)} mbb</span>
      )}
    </span>
  );
}

function Spot({ e }: { e: LibraryEntry }) {
  if (e.kind === "postflop") {
    return (
      <span className="flex flex-wrap items-center gap-2">
        {e.board && <CardRow cards={e.board} size="sm" />}
        <span className="font-mono text-muted tabular">
          pot {formatBb(e.starting_pot_mbb ?? 0)}bb · stack {formatBb(e.effective_stack_mbb ?? 0)}bb
        </span>
      </span>
    );
  }
  return (
    <span className="font-mono text-muted tabular">
      Preflop · {e.players ?? "?"} players · {formatBb(e.effective_stack_mbb ?? 0)}bb
    </span>
  );
}

/** Every saved result, filterable by kind; open one to browse it. */
export function Library() {
  const [kind, setKind] = useState<Kind>("all");
  const [open, setOpen] = useState<string | null>(null);
  const { data, error, loading } = useApi<LibraryEntry[]>(
    `/api/v1/solver/library?limit=200${kind === "all" ? "" : `&kind=${kind}`}`,
  );
  return (
    <div className="flex flex-col gap-3">
      <div role="tablist" aria-label="Kind of result" className="flex gap-1">
        {(["all", "postflop", "preflop"] as const).map((k) => (
          <button
            key={k}
            role="tab"
            type="button"
            aria-selected={kind === k}
            onClick={() => setKind(k)}
            className={cn(
              "rounded-md px-3 py-1 text-sm capitalize",
              kind === k ? "bg-surface-3 text-fg" : "text-muted hover:bg-surface-2",
            )}
          >
            {k}
          </button>
        ))}
      </div>
      {error ? (
        <ErrorState message={error.message} />
      ) : loading && !data ? (
        <LoadingState label="Loading saved results…" />
      ) : !data || data.length === 0 ? (
        <EmptyState
          title="No saved results"
          description="Results appear here when a solve on the Solver or Preflop page finishes."
        />
      ) : (
        <ul className="divide-y divide-line" aria-label="Saved results">
          {libraryRows(data).map((row) => {
            if (row.kind === "hand") {
              const isOpen = open === row.key;
              const h = row.entries[0].hand!;
              return (
                <li key={row.key} className="py-1.5">
                  <button
                    type="button"
                    aria-expanded={isOpen}
                    onClick={() => setOpen(isOpen ? null : row.key)}
                    className="flex w-full flex-wrap items-center gap-x-4 gap-y-1 rounded px-1 py-1.5 text-left text-xs hover:bg-surface-2"
                  >
                    <ChevronRight
                      aria-hidden
                      className={cn("size-3.5 shrink-0 text-subtle transition-transform", isOpen && "rotate-90")}
                    />
                    <HandRow entries={row.entries} />
                  </button>
                  {isOpen && (
                    <div className="mt-2 pb-3 pl-6">
                      <WholeHand sessionId={h.session_id} handNo={h.hand_no} />
                    </div>
                  )}
                </li>
              );
            }
            const e = row.e;
            const isOpen = open === e.job_id;
            return (
              <li key={e.job_id} className="py-1.5">
                <button
                  type="button"
                  aria-expanded={isOpen}
                  onClick={() => setOpen(isOpen ? null : e.job_id)}
                  className="flex w-full flex-wrap items-center gap-x-4 gap-y-1 rounded px-1 py-1.5 text-left text-xs hover:bg-surface-2"
                >
                  <ChevronRight
                    aria-hidden
                    className={cn("size-3.5 shrink-0 text-subtle transition-transform", isOpen && "rotate-90")}
                  />
                  <Spot e={e} />
                  <Quality e={e} />
                  <span className="ml-auto font-mono text-subtle tabular">
                    {e.iterations.toLocaleString()} it · {e.solver_backend}@{e.solver_version.slice(0, 7)} · engine{" "}
                    {e.engine_version}
                    {e.finished_at && ` · ${new Date(e.finished_at).toLocaleString()}`}
                  </span>
                </button>
                {isOpen && (
                  <div className="mt-2 pb-3 pl-6">
                    {e.kind === "postflop" && e.ranges && (
                      <dl className="mb-3 grid gap-1 text-xs">
                        {e.ranges.map((r, i) => (
                          <div key={i} className="flex gap-2">
                            <dt className="w-8 shrink-0 text-muted">{i === 0 ? "OOP" : "IP"}</dt>
                            <dd className="font-mono break-all text-fg">{r}</dd>
                          </div>
                        ))}
                      </dl>
                    )}
                    {e.kind === "postflop" ? <ResultView jobId={e.job_id} /> : <PreflopResultView jobId={e.job_id} />}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

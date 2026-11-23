"use client";

import { ChevronRight, CircleAlert, CircleCheck, TriangleAlert } from "lucide-react";
import { useState } from "react";
import type { PreflopNode, PreflopResult } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBb } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { Metric } from "@/components/solver/ResultView";
import { StrategyMatrix } from "@/components/solver/StrategyMatrix";
import { actionColors } from "@/components/solver/actionColors";

const signedBb = (mbb: number) => `${mbb >= 0 ? "+" : "−"}${formatBb(Math.round(Math.abs(mbb)))}bb`;

/** The banner every preflop result carries: validated push/fold, or an approximate model. */
export function ScopeBanner({ scope }: { scope: PreflopResult["validated_scope"] }) {
  if (scope === "heads_up_push_fold") {
    return (
      <p className="flex items-start gap-2 rounded-md border border-accent/30 bg-accent-dim px-3 py-2 text-sm text-accent-strong">
        <CircleCheck aria-hidden className="mt-0.5 size-4 shrink-0" />
        Heads-up push/fold. RiverLab has checked GTOpen against its own exact equilibrium for this tree (at most
        0.00002bb/hand exploitable at 10bb and 20bb).
      </p>
    );
  }
  return (
    <p
      role="note"
      className="flex items-start gap-2 rounded-md border border-warn/30 bg-warn-dim px-3 py-2 text-sm text-warn"
    >
      <TriangleAlert aria-hidden className="mt-0.5 size-4 shrink-0" />
      <span>
        <strong className="font-semibold">Approximate model, not a solved game.</strong> Pots that reach the flop are
        valued by a model instead of being played out, and RiverLab has not validated this tree. Use it to explore how
        sizes and stacks change ranges, not as an equilibrium.
      </span>
    </p>
  );
}

function rangeFrequency(node: PreflopNode, a: number): number {
  const total = node.by_class.reduce((s, c) => s + c.weight, 0) || 1;
  return node.by_class.reduce((s, c) => s + c.weight * c.frequencies[a], 0) / total;
}

/** Walks a saved preflop tree by action. */
export function PreflopBrowser({ jobId }: { jobId: string }) {
  const [path, setPath] = useState<number[]>([]);
  const key = path.join(".");
  const fetched = useApi<PreflopNode>(`/api/v1/solver/jobs/${jobId}/preflop/node?path=${key}`);
  const node = fetched.data?.path === key ? fetched.data : null;
  const labels = node?.history.slice(0, path.length).map((h, i) => `${h.actor} ${h.actions[path[i]] ?? ""}`) ?? [];

  return (
    <div className="flex flex-col gap-3">
      <nav aria-label="Path in the preflop tree" className="flex flex-wrap items-center gap-1 text-xs">
        <button
          type="button"
          className="rounded px-1.5 py-0.5 text-accent hover:bg-surface-2"
          onClick={() => setPath([])}
        >
          Start
        </button>
        {labels.map((label, i) => (
          <span key={i} className="flex items-center gap-1">
            <ChevronRight aria-hidden className="size-3 text-subtle" />
            <button
              type="button"
              aria-current={i === labels.length - 1 ? "step" : undefined}
              className={cn(
                "rounded px-1.5 py-0.5 hover:bg-surface-2",
                i === labels.length - 1 ? "text-fg" : "text-accent",
              )}
              onClick={() => setPath(path.slice(0, i + 1))}
            >
              {label}
            </button>
          </span>
        ))}
      </nav>
      {fetched.error ? (
        <ErrorState message={fetched.error.message} />
      ) : !node ? (
        <LoadingState label="Loading node…" />
      ) : (
        <NodeDetails node={node} onAction={(a) => setPath([...path, a])} />
      )}
    </div>
  );
}

function NodeDetails({ node, onAction }: { node: PreflopNode; onAction: (a: number) => void }) {
  const labels = node.actions.map((a) => a.label);
  const colors = actionColors(labels);
  return (
    <div className="flex flex-col gap-3">
      <p className="font-mono text-xs text-muted tabular">
        pot {formatBb(Math.round(node.pot_mbb))}bb · reach {(node.reach_probability * 100).toFixed(2)}% (model)
        {node.stack_to_pot !== null && ` · SPR ${node.stack_to_pot.toFixed(1)}`}
      </p>
      {node.strategy_note && (
        <p className="flex items-start gap-2 text-xs text-warn">
          <CircleAlert aria-hidden className="mt-px size-3.5 shrink-0" />
          {node.strategy_note}
        </p>
      )}
      {node.kind === "action" && node.actor && (
        <>
          <p className="text-sm text-fg">{node.actor} to act — choose an action to continue:</p>
          <div className="flex flex-wrap gap-2">
            {node.actions.map((a, i) => (
              <Button
                key={a.label}
                size="sm"
                onClick={() => onAction(i)}
                icon={<span aria-hidden className="size-2.5 rounded-sm" style={{ background: colors[i] }} />}
              >
                {a.label} <span className="font-mono text-muted">{(rangeFrequency(node, i) * 100).toFixed(1)}%</span>
              </Button>
            ))}
          </div>
          <StrategyMatrix actions={labels} byClass={node.by_class} />
        </>
      )}
      {node.kind !== "action" && (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-fg">
            {node.kind === "fold_win"
              ? "Everyone else folded; the hand ends here."
              : "The hand continues to the flop (or is all-in). These are the ranges that get here; the model, not a solve, values this pot."}
          </p>
          <div className="grid gap-4 lg:grid-cols-2">
            {node.ranges.map((r) => (
              <div key={r.position} className="flex flex-col gap-1.5">
                <h4 className="text-xs font-semibold text-fg">{r.position} range</h4>
                <StrategyMatrix actions={["Share of combos in range"]} byClass={r.classes} />
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function PreflopResultView({ jobId }: { jobId: string }) {
  const { data: r, error, loading } = useApi<PreflopResult>(`/api/v1/solver/jobs/${jobId}/preflop`);
  if (error) return <ErrorState message={error.message} />;
  if (loading || !r) return <LoadingState label="Loading result…" />;
  return (
    <div className="flex flex-col gap-4">
      <ScopeBanner scope={r.validated_scope} />
      <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
        <Metric
          label="Model gap"
          value={`${r.model_gap_mbb.toFixed(2)} mbb/hand`}
          sub="Best-response gain summed over players, inside the model"
        />
        <Metric
          label="Iterations"
          value={r.iterations.toLocaleString()}
          sub={`${r.stop_reason.replaceAll("_", " ")} · ${(r.elapsed_ms / 1000).toFixed(1)} s`}
        />
        <Metric
          label="Tree"
          value={`${r.nodes.toLocaleString()} nodes`}
          sub={`${(r.memory_bytes / 1e6).toFixed(1)} MB`}
        />
        <Metric label="Backend" value={`${r.solver_backend} @ ${r.solver_version.slice(0, 10)}`} sub={r.model} />
      </div>
      <div className="flex flex-wrap gap-2" aria-label="Values per position (model)">
        {r.positions.map((p, i) => (
          <span key={p} className="rounded border border-line px-2 py-1 font-mono text-xs text-muted tabular">
            {p} {signedBb(r.values_mbb[i])}
          </span>
        ))}
        <span className="self-center text-xs text-subtle">per hand under the model, blinds included</span>
      </div>
      <ul className="flex flex-col gap-1">
        {r.model_notes.map((n) => (
          <li key={n} className="flex items-start gap-2 text-xs text-muted">
            <CircleAlert aria-hidden className="mt-px size-3.5 shrink-0 text-warn" />
            {n}
          </li>
        ))}
      </ul>
      <div className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold text-fg">Strategy tree</h3>
        {r.tree_available ? (
          <PreflopBrowser jobId={jobId} />
        ) : (
          <>
            <p className="text-xs text-muted">The saved tree is not available; only the first decision is shown.</p>
            <NodeDetails node={r.root} onAction={() => undefined} />
          </>
        )}
      </div>
    </div>
  );
}

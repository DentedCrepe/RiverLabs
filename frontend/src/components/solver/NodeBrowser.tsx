"use client";

import { ChevronRight } from "lucide-react";
import { useState } from "react";
import type { NodeView } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBb } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { CardRow, PlayingCard } from "@/components/poker/CardText";
import { cn } from "@/components/ui/cn";
import { actionColors } from "./actionColors";
import { StrategyMatrix } from "./StrategyMatrix";

type Step = { token: string; label: string };
const PLAYER = ["OOP", "IP"];

function overall(node: NodeView): number[] {
  const total = node.by_class.reduce((s, c) => s + c.weight, 0) || 1;
  return node.actions.map((_, a) => node.by_class.reduce((s, c) => s + c.weight * c.frequencies[a], 0) / total);
}

/** Walks a saved solve tree: actions at decisions, dealt cards at chance nodes. */
export function NodeBrowser({ jobId }: { jobId: string }) {
  const [steps, setSteps] = useState<Step[]>([]);
  const path = steps.map((s) => s.token).join(".");
  const {
    data: node,
    error,
    loading,
  } = useApi<NodeView>(`/api/v1/solver/jobs/${jobId}/node?path=${encodeURIComponent(path)}`);
  const go = (step: Step) => setSteps((s) => [...s, step]);

  return (
    <div className="flex flex-col gap-3">
      <nav aria-label="Path in the game tree" className="flex flex-wrap items-center gap-1 text-xs">
        <button
          type="button"
          className="rounded px-1.5 py-0.5 text-accent hover:bg-surface-2"
          onClick={() => setSteps([])}
        >
          Root
        </button>
        {steps.map((s, i) => (
          <span key={i} className="flex items-center gap-1">
            <ChevronRight aria-hidden className="size-3 text-subtle" />
            <button
              type="button"
              aria-current={i === steps.length - 1 ? "step" : undefined}
              className={cn(
                "rounded px-1.5 py-0.5 hover:bg-surface-2",
                i === steps.length - 1 ? "text-fg" : "text-accent",
              )}
              onClick={() => setSteps(steps.slice(0, i + 1))}
            >
              {s.label}
            </button>
          </span>
        ))}
      </nav>
      {error ? (
        <ErrorState message={error.message} />
      ) : loading && !node ? (
        <LoadingState label="Loading node…" />
      ) : node ? (
        <div className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center gap-3 text-sm">
            <CardRow cards={node.board} size="sm" />
            <span className="text-muted capitalize">{node.street}</span>
            <span className="font-mono text-xs text-muted tabular">
              pot {formatBb(Math.round(node.put_mbb[0] + node.put_mbb[1]))}bb · reach OOP{" "}
              {(node.reach_fraction[0] * 100).toFixed(1)}% · IP {(node.reach_fraction[1] * 100).toFixed(1)}%
            </span>
          </div>
          {node.kind === "decision" && node.player !== null && (
            <>
              <p className="text-sm text-fg">{PLAYER[node.player]} to act — choose an action to continue:</p>
              <div className="flex flex-wrap gap-2">
                {node.actions.map((a, i) => {
                  const f = overall(node)[i];
                  return (
                    <Button
                      key={a}
                      size="sm"
                      onClick={() => go({ token: `a${i}`, label: a })}
                      icon={
                        <span
                          aria-hidden
                          className="size-2.5 rounded-sm"
                          style={{ background: actionColors(node.actions)[i] }}
                        />
                      }
                    >
                      {a} <span className="font-mono text-muted">{(f * 100).toFixed(1)}%</span>
                    </Button>
                  );
                })}
              </div>
              <StrategyMatrix actions={node.actions} byClass={node.by_class} />
            </>
          )}
          {node.kind === "chance" && (
            <div className="flex flex-col gap-2">
              <p className="text-sm text-fg">Deal the {node.board.length === 3 ? "turn" : "river"}:</p>
              <div className="flex flex-wrap gap-1.5" role="group" aria-label="Cards that can be dealt">
                {node.cards.map((c) => (
                  <button
                    key={c}
                    type="button"
                    className="rounded hover:ring-2 hover:ring-accent"
                    onClick={() => go({ token: `c${c}`, label: c })}
                  >
                    <PlayingCard card={c} size="sm" />
                  </button>
                ))}
              </div>
            </div>
          )}
          {(node.kind === "fold" || node.kind === "showdown") && (
            <p className="text-sm text-muted">
              {node.kind === "fold" && node.player !== null ? `${PLAYER[node.player]} folds.` : "Showdown."} End of the
              hand in this line.
            </p>
          )}
        </div>
      ) : null}
    </div>
  );
}

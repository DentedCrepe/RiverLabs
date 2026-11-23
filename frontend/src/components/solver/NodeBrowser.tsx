"use client";

import { ChevronRight } from "lucide-react";
import { useState } from "react";
import { type LineStep, matchAction } from "@/lib/solve";
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

/** A real line to follow from the root, ending at your decision (from "Solve this spot"). */
export interface Follow {
  line: LineStep[];
  heroPlayer: number;
  heroAction: LineStep;
  heroClass: string;
  heroCards: string[];
}

/** The solver's mix for your hand at this node, with the action you took marked. */
function YourHand({ node, follow, atDecision }: { node: NodeView; follow: Follow; atDecision: boolean }) {
  const c = node.by_class.find((x) => x.label === follow.heroClass);
  if (!c) return null;
  const taken = atDecision ? matchAction(node.actions, follow.heroAction) : null;
  const colors = actionColors(node.actions);
  return (
    <div className="flex flex-col gap-1.5 rounded-md border border-accent/40 bg-accent-dim/40 px-3 py-2">
      <p className="flex flex-wrap items-center gap-2 text-xs font-semibold text-fg">
        Your hand <CardRow cards={follow.heroCards} size="sm" /> — solver mix for {c.label}
      </p>
      {node.actions.map((a, i) => (
        <div key={a} className="flex items-center gap-2 text-xs">
          <span className={cn("w-28 shrink-0 truncate", taken === i ? "font-semibold text-fg" : "text-muted")}>
            {a}
            {taken === i && " ← you"}
          </span>
          <span className="h-2 flex-1 overflow-hidden rounded-full bg-surface-3">
            <span className="block h-full" style={{ width: `${c.frequencies[i] * 100}%`, background: colors[i] }} />
          </span>
          <span className="w-12 text-right font-mono text-muted tabular">{(c.frequencies[i] * 100).toFixed(1)}%</span>
        </div>
      ))}
      <p className="text-[11px] text-subtle">
        Averaged over the {c.label} combos still in your range; an approximate equilibrium for this tree&apos;s bet
        sizes and these ranges.
      </p>
    </div>
  );
}

/** Walks a saved solve tree: actions at decisions, dealt cards at chance nodes. */
export function NodeBrowser({ jobId, follow }: { jobId: string; follow?: Follow }) {
  const [steps, setSteps] = useState<Step[]>([]);
  // Following the real line until it reaches your decision or you navigate yourself.
  const [auto, setAuto] = useState(Boolean(follow));
  const [lost, setLost] = useState<string | null>(null);
  const path = steps.map((s) => s.token).join(".");
  const {
    data: node,
    error,
    loading,
  } = useApi<NodeView>(`/api/v1/solver/jobs/${jobId}/node?path=${encodeURIComponent(path)}`);
  const go = (step: Step) => {
    setAuto(false);
    setSteps((s) => [...s, step]);
  };
  const jump = (next: Step[]) => {
    setAuto(false);
    setSteps(next);
  };
  const loaded = node && node.path === path ? node : null;

  // Adjusting state while rendering (not in an effect): each fetched node advances one step.
  if (auto && follow && loaded && steps.length < follow.line.length) {
    const step = follow.line[steps.length];
    const a = loaded.kind === "decision" && loaded.player === step.player ? matchAction(loaded.actions, step) : null;
    if (a === null) {
      setAuto(false);
      setLost(`The tree has no "${step.kind}" here, so the real line could not be followed further.`);
    } else {
      setSteps([...steps, { token: `a${a}`, label: loaded.actions[a] }]);
    }
  }
  const atDecision = Boolean(follow && auto && loaded && steps.length === follow.line.length);

  return (
    <div className="flex flex-col gap-3">
      <nav aria-label="Path in the game tree" className="flex flex-wrap items-center gap-1 text-xs">
        <button type="button" className="rounded px-1.5 py-0.5 text-accent hover:bg-surface-2" onClick={() => jump([])}>
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
              onClick={() => jump(steps.slice(0, i + 1))}
            >
              {s.label}
            </button>
          </span>
        ))}
      </nav>
      {lost && <p className="text-xs text-warn">{lost}</p>}
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
              {atDecision && (
                <p className="text-sm font-medium text-accent-strong">
                  Your decision in the hand ({PLAYER[node.player]}).
                </p>
              )}
              {follow && node.player === follow.heroPlayer && (
                <YourHand node={node} follow={follow} atDecision={atDecision} />
              )}
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

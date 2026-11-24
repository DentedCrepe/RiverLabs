"use client";

import { ChevronRight, Lock } from "lucide-react";
import { useState } from "react";
import { type LineStep, lockFrequencies, matchAction } from "@/lib/solve";
import type { NodeLock, NodeView } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBb } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { CardRow, PlayingCard } from "@/components/poker/CardText";
import { cn } from "@/components/ui/cn";
import { actionColors } from "./actionColors";
import { StrategyMatrix } from "./StrategyMatrix";

type Step = { token: string; label: string };

const sameLockPath = (a: number[], b: number[]) => a.length === b.length && a.every((x, i) => x === b[i]);
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

/**
 * Lock this decision: fix how often the acting player takes each action across their range, then
 * solve again so the other player's strategy becomes a response to it.
 */
function LockPanel({
  node,
  path,
  existing,
  onLock,
}: {
  node: NodeView;
  path: number[];
  existing: NodeLock | undefined;
  onLock: (path: number[], frequencies: number[]) => Promise<void>;
}) {
  const start = (existing?.frequencies ?? overall(node)).map((f) => (f * 100).toFixed(1));
  const [percent, setPercent] = useState<string[]>(start);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  async function submit() {
    const r = lockFrequencies(percent);
    if (!r.ok) return setError(r.error);
    setError(null);
    setBusy(true);
    try {
      await onLock(path, r.value);
      setDone(true);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <details className="rounded-md border border-line px-3 py-2">
      <summary className="flex cursor-pointer items-center gap-2 text-sm text-accent">
        <Lock aria-hidden className="size-3.5" /> Lock this decision
      </summary>
      <div className="mt-2 flex flex-col gap-2">
        <p className="text-xs text-muted">
          Set how often {PLAYER[node.player ?? 0]} takes each action here, across their whole range (for example, an
          opponent who bets too often). RiverLab solves the spot again with these frequencies fixed, so the other
          player&apos;s strategy becomes the best response to them. Each hand keeps its relative preferences from this
          solve.
        </p>
        <div className="flex flex-wrap gap-3">
          {node.actions.map((a, i) => (
            <label key={a} className="flex items-center gap-1.5 text-xs text-fg">
              {a}
              <input
                type="number"
                min={0}
                max={100}
                step={0.1}
                inputMode="decimal"
                aria-label={`${a} (%)`}
                value={percent[i]}
                onChange={(e) => setPercent((p) => p.map((x, k) => (k === i ? e.target.value : x)))}
                className="h-7 w-20 rounded-md border border-line bg-canvas px-2 font-mono text-xs tabular"
              />
              %
            </label>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <Button size="sm" variant="primary" loading={busy} onClick={submit} icon={<Lock className="size-3.5" />}>
            Solve with this lock
          </Button>
          {done && (
            <span role="status" className="text-xs text-accent">
              Queued. The locked solve opens here when it finishes.
            </span>
          )}
        </div>
        {error && (
          <p role="alert" className="text-xs text-danger">
            {error}
          </p>
        )}
      </div>
    </details>
  );
}

/** Walks a saved solve tree: actions at decisions, dealt cards at chance nodes. */
export function NodeBrowser({
  jobId,
  follow,
  locks = [],
  onLock,
}: {
  jobId: string;
  follow?: Follow;
  /** Locks the solve was run with: locked decisions are marked. */
  locks?: NodeLock[];
  /** Offers "Lock this decision" at decisions on the first street, when given. */
  onLock?: (path: number[], frequencies: number[]) => Promise<void>;
}) {
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
  // Locks apply to decisions reached by actions only (the first street of the solve).
  const actionPath = steps.every((s) => s.token.startsWith("a")) ? steps.map((s) => Number(s.token.slice(1))) : null;
  const lockedHere = actionPath !== null && locks.some((l) => sameLockPath(l.path, actionPath));

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
              {lockedHere && (
                <p className="flex items-center gap-2 text-sm text-warn">
                  <Lock aria-hidden className="size-3.5" /> Locked: {PLAYER[node.player]} plays these frequencies
                  whatever the other player does.
                </p>
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
              {onLock && actionPath && loaded && (
                <LockPanel
                  key={path}
                  node={loaded}
                  path={actionPath}
                  existing={locks.find((l) => sameLockPath(l.path, actionPath))}
                  onLock={onLock}
                />
              )}
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

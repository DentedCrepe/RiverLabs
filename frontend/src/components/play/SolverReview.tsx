"use client";

import { CircleAlert, Cpu, LoaderCircle } from "lucide-react";
import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { ApiError, api } from "@/lib/api";
import type { DecisionReview, HandSolverReview, ReviewStreet } from "@/lib/play";
import { Button } from "@/components/ui/Button";
import { ErrorState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { actionColors } from "@/components/solver/actionColors";

const POLL_MS = 2000;

/** The hand's solver review: loaded if it exists, polled while it runs, started on request. */
export function useSolverReview(sessionId: string | undefined, handNo: number) {
  const [review, setReview] = useState<HandSolverReview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const [tick, setTick] = useState(0);
  const url = sessionId ? `/api/v1/play/sessions/${sessionId}/hands/${handNo}/review` : null;

  useEffect(() => {
    if (!url) return;
    let cancelled = false;
    api
      .get<HandSolverReview>(url, { poll: tick > 0 })
      .then((r) => !cancelled && setReview(r))
      .catch((e: unknown) => {
        // 404: no review yet. Anything else is shown.
        if (!cancelled && !(e instanceof ApiError && e.status === 404)) setError(String((e as Error).message ?? e));
      });
    return () => {
      cancelled = true;
    };
  }, [url, tick]);

  const running = review?.status === "queued" || review?.status === "running";
  useEffect(() => {
    if (!running) return;
    const t = setTimeout(() => setTick((x) => x + 1), POLL_MS);
    return () => clearTimeout(t);
  }, [running, tick]);

  const start = useCallback(async () => {
    if (!url) return;
    setStarting(true);
    setError(null);
    try {
      setReview(await api.post<HandSolverReview>(url, {}));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setStarting(false);
    }
  }, [url]);

  return { review, error, starting, start };
}

function streetLine(s: ReviewStreet): string {
  switch (s.status) {
    case "pending":
      return "waiting";
    case "solving": {
      const p = s.progress;
      return p?.iterations
        ? `solving · ${p.iterations} iterations · ${p.exploitability_pct_pot?.toFixed(2) ?? "?"}% pot exploitable`
        : `solving (${s.job_status ?? "queued"})`;
    }
    case "done":
      return s.review?.followed ? "solved" : "solved, but the real line left the tree";
    case "failed":
      return s.error ?? "failed";
    default:
      return "not solvable";
  }
}

/** Summary of the review: status per street, total EV lost, and the start button. */
export function SolverReviewPanel({
  review,
  error,
  starting,
  onStart,
}: {
  review: HandSolverReview | null;
  error: string | null;
  starting: boolean;
  onStart: () => void;
}) {
  const decisions = review?.streets.flatMap((s) => s.review?.decisions ?? []) ?? [];
  const lost = decisions.reduce((a, d) => a + (d.ev_loss_bb ?? 0), 0);
  const retry = review?.status === "failed" || review?.status === "interrupted";
  return (
    <section
      aria-label="Solver review"
      className="flex flex-col gap-2 rounded-md border border-accent/30 bg-accent-dim/30 px-3 py-2 text-xs"
    >
      {!review || retry ? (
        <div className="flex flex-col gap-1.5">
          {retry && (
            <p className="text-warn">
              The last review{" "}
              {review?.status === "failed" ? `failed: ${review?.error_message ?? ""}` : "was interrupted"}.
            </p>
          )}
          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="sm"
              variant="primary"
              loading={starting}
              onClick={onStart}
              icon={<Cpu className="size-3.5" />}
            >
              Review this hand with the solver
            </Button>
            <span className="text-muted">
              Solves each street you played heads-up after the flop, one after the other. A flop can take a few minutes.
            </span>
          </div>
        </div>
      ) : (
        <>
          <p className="flex flex-wrap items-center gap-2 font-medium text-fg">
            <Cpu aria-hidden className="size-3.5 text-accent" />
            Solver review
            {review.status === "done" ? (
              <span className="font-mono text-muted tabular">
                {decisions.length} decision{decisions.length === 1 ? "" : "s"} · EV lost {lost.toFixed(2)}bb
              </span>
            ) : (
              <LoaderCircle aria-hidden className="size-3.5 animate-spin text-accent" />
            )}
          </p>
          <ul className="flex flex-col gap-0.5" aria-label="Streets">
            {review.streets.map((s) => (
              <li key={s.street} className="flex flex-wrap gap-2">
                <span className="w-10 text-fg capitalize">{s.street}</span>
                <span className={cn(s.status === "failed" ? "text-danger" : "text-muted")}>{streetLine(s)}</span>
                {s.job_id && s.status === "done" && (
                  <Link href="/library" className="text-accent hover:text-accent-strong">
                    full solve
                  </Link>
                )}
              </li>
            ))}
          </ul>
          {review.streets.some((s) => s.notes?.length || s.review?.notes.length || s.note) && (
            <details>
              <summary className="cursor-pointer text-muted">What these solves assume</summary>
              <ul className="mt-1 flex flex-col gap-1">
                {review.streets.flatMap((s) =>
                  [...(s.note ? [s.note] : []), ...(s.notes ?? []), ...(s.review?.notes ?? [])].map((n, i) => (
                    <li key={`${s.street}-${i}`} className="flex items-start gap-1.5 text-muted">
                      <CircleAlert aria-hidden className="mt-px size-3 shrink-0 text-warn" />
                      <span>
                        <span className="text-fg capitalize">{s.street}:</span> {n}
                      </span>
                    </li>
                  )),
                )}
              </ul>
            </details>
          )}
          <p className="text-[11px] text-subtle">
            Each street is an approximate equilibrium for its tree&apos;s bet sizes, with its exploitability shown in
            the full solve. EVs assume both players follow the solution from that point.
          </p>
        </>
      )}
      {error && <ErrorState message={error} />}
    </section>
  );
}

/** The solver's view of one of your decisions: its mix for your exact hand and each action's EV. */
export function SolverDecision({ r }: { r: DecisionReview }) {
  const colors = actionColors(r.actions);
  const evs = r.action_evs_bb;
  const fmt = (x: number) => `${x >= 0 ? "+" : "−"}${Math.abs(x).toFixed(2)}bb`;
  return (
    <div className="flex flex-col gap-1" aria-label="Solver view of this decision">
      <p className="text-[11px] text-muted">
        Solver, your exact hand
        {r.ev_loss_bb !== null && (
          <span className={cn("ml-2 font-mono tabular", r.ev_loss_bb > 0.005 ? "text-danger" : "text-accent-strong")}>
            {r.ev_loss_bb > 0.005 ? `EV lost ${r.ev_loss_bb.toFixed(2)}bb` : "no EV lost"}
          </span>
        )}
      </p>
      {r.actions.map((a, i) => (
        <div key={a} className="flex items-center gap-2 text-xs">
          <span className={cn("w-28 shrink-0 truncate", r.taken === i ? "font-semibold text-fg" : "text-muted")}>
            {a}
            {r.taken === i && " ← you"}
            {r.best === i && " ★"}
          </span>
          <span className="h-2 flex-1 overflow-hidden rounded-full bg-surface-3">
            <span
              className="block h-full"
              style={{ width: `${(r.strategy?.[i] ?? 0) * 100}%`, background: colors[i] }}
            />
          </span>
          <span className="w-10 text-right font-mono text-muted tabular">
            {r.strategy ? `${(r.strategy[i] * 100).toFixed(0)}%` : "—"}
          </span>
          <span className="w-16 text-right font-mono text-muted tabular">{evs ? fmt(evs[i]) : "—"}</span>
        </div>
      ))}
    </div>
  );
}

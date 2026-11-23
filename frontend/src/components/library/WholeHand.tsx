"use client";

import { useState } from "react";
import { type HandRecord, type HandSolverReview, type ReviewStreet, describeAction } from "@/lib/play";
import { useApi } from "@/lib/useApi";
import { HandReview } from "@/components/play/HandHistory";
import type { Follow } from "@/components/solver/NodeBrowser";
import { ResultView } from "@/components/solver/ResultView";
import { Button } from "@/components/ui/Button";
import { ErrorState, LoadingState } from "@/components/ui/States";

type Open = { jobId: string; follow?: Follow; label: string };

/** Your decisions on a street, each with the real line up to it, for opening the solve there. */
function decisionsOf(st: ReviewStreet, h: HandRecord): { label: string; follow: Follow }[] {
  if (!st.line || st.hero_player === undefined || !st.hero_combo || !st.hero_class) return [];
  const units = h.tournament ? "chips" : "bb";
  return st.line.flatMap((step, i) => {
    if (step.decision === null || step.player !== st.hero_player) return [];
    const d = h.hero_decisions[step.decision];
    return [
      {
        label: `You ${describeAction(d.action, units, false, true)}`,
        follow: {
          line: st.line!.slice(0, i),
          heroPlayer: st.hero_player!,
          heroAction: step,
          heroClass: st.hero_class!,
          heroCards: [st.hero_combo!.slice(0, 2), st.hero_combo!.slice(2, 4)],
        },
      },
    ];
  });
}

/**
 * A hand you played, with its street solves: the replay (the solver's view under each of your
 * decisions) and, per street, the solve opened at each of your decisions.
 */
export function WholeHand({ sessionId, handNo }: { sessionId: string; handNo: number }) {
  const base = `/api/v1/play/sessions/${sessionId}/hands/${handNo}`;
  const hand = useApi<HandRecord>(base);
  const review = useApi<HandSolverReview>(`${base}/review`);
  const [open, setOpen] = useState<Open | null>(null);
  if (hand.error) return <ErrorState message={hand.error.message} />;
  if (!hand.data) return <LoadingState label="Loading the hand…" />;
  const h = hand.data;
  const solved = (review.data?.streets ?? []).filter((s) => s.status === "done" && s.job_id);
  return (
    <div className="flex flex-col gap-4">
      <HandReview hand={h} sessionId={sessionId} />
      {solved.length > 0 && (
        <section aria-label="Street solves" className="flex flex-col gap-2">
          <h4 className="text-xs font-semibold text-fg">Open a street&apos;s solve</h4>
          {solved.map((st) => (
            <div key={st.street} className="flex flex-wrap items-center gap-2 text-xs">
              <span className="w-10 text-fg capitalize">{st.street}</span>
              {decisionsOf(st, h).map((d, i) => (
                <Button
                  key={i}
                  size="sm"
                  variant="secondary"
                  onClick={() => setOpen({ jobId: st.job_id!, follow: d.follow, label: `${st.street}: ${d.label}` })}
                >
                  At “{d.label}”
                </Button>
              ))}
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setOpen({ jobId: st.job_id!, label: `${st.street}: from the start` })}
              >
                From the start
              </Button>
            </div>
          ))}
        </section>
      )}
      {open && (
        <section aria-label="Street solve" className="flex flex-col gap-2 rounded-md border border-line p-3">
          <h4 className="text-xs font-semibold text-fg capitalize">{open.label}</h4>
          <ResultView key={`${open.jobId}-${open.label}`} jobId={open.jobId} follow={open.follow} />
        </section>
      )}
    </div>
  );
}

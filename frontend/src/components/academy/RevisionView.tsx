"use client";

import Link from "next/link";
import { type Revision, conceptName } from "@/lib/academy";
import { useApi } from "@/lib/useApi";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { QuestionCard } from "./QuestionCard";

/**
 * Questions due for revision. A wrong answer is due straight away; after consecutive right
 * answers a question comes back in 1, 3, 7, 21 and then 60 days (docs/academy.md).
 */
export function RevisionView() {
  const { data, error, loading, reload } = useApi<Revision>("/api/v1/academy/revision");
  if (error) return <ErrorState message={error.message} />;
  if (loading && !data) return <LoadingState />;
  const r = data!;
  return (
    <div className="flex flex-col gap-4">
      {r.misconceptions.length > 0 && (
        <p className="text-sm text-muted">
          Concepts you have missed more than once: {r.misconceptions.map((m) => conceptName(m.concept)).join(", ")}.
        </p>
      )}
      {r.due.length === 0 ? (
        <EmptyState
          title="Nothing due"
          description="Answer lesson questions; anything you get wrong comes back here, and right answers return at growing intervals."
        />
      ) : (
        r.due.map((d) => (
          <div key={d.question.id} className="flex flex-col gap-1">
            <p className="text-xs text-muted">
              From{" "}
              <Link href={`/academy/${d.question.id.split(":")[0]}`} className="text-accent hover:underline">
                {d.question.id.split(":")[0].replaceAll("-", " ")}
              </Link>
              {d.last_correct ? " · due for review" : " · answered wrongly last time"}
            </p>
            <QuestionCard question={d.question} onAnswered={() => setTimeout(reload, 4000)} />
          </div>
        ))
      )}
    </div>
  );
}

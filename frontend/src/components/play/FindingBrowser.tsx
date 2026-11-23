"use client";

import { ChevronLeft, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { HandRecord, PlayReport } from "@/lib/play";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { HandReview } from "./HandHistory";

type Example = PlayReport["findings"][number]["examples"][number];

/** Flick through the hands behind one weakness, each with its full replay. */
export function FindingBrowser({ examples, sessionId }: { examples: Example[]; sessionId?: string }) {
  const [i, setI] = useState(0);
  const ex = examples[i];
  const sid = ex.session_id ?? sessionId;
  const { data, error } = useApi<HandRecord>(sid ? `/api/v1/play/sessions/${sid}/hands/${ex.hand_no}` : null);
  const hand = data && data.hand_no === ex.hand_no ? data : null;
  return (
    <div className="flex flex-col gap-2" aria-label="Hands with this weakness">
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <Button
          size="sm"
          variant="secondary"
          disabled={i === 0}
          onClick={() => setI(i - 1)}
          icon={<ChevronLeft className="size-3.5" />}
          aria-label="Previous hand"
        />
        <span className="font-mono text-muted tabular">
          {i + 1} of {examples.length}
        </span>
        <Button
          size="sm"
          variant="secondary"
          disabled={i === examples.length - 1}
          onClick={() => setI(i + 1)}
          icon={<ChevronRight className="size-3.5" />}
          aria-label="Next hand"
        />
        <span className="text-fg">
          Hand {ex.hand_no}, {ex.street}: {ex.text}
        </span>
      </div>
      {!sid ? (
        <p className="text-xs text-muted">This hand&apos;s game is not known, so it cannot be opened.</p>
      ) : error ? (
        <ErrorState message={error.message} />
      ) : !hand ? (
        <LoadingState label="Loading the hand…" />
      ) : (
        <div className="rounded-md border border-line p-3">
          <HandReview hand={hand} sessionId={sid} />
        </div>
      )}
    </div>
  );
}

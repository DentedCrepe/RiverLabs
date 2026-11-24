"use client";

import { useState } from "react";
import { cn } from "@/components/ui/cn";
import { PostflopTrainer } from "./PostflopTrainer";
import { PreflopTrainer } from "./PreflopTrainer";
import { PushFoldTrainer } from "./PushFoldTrainer";

const TABS = [
  ["push-fold", "Push/fold"],
  ["preflop", "Preflop"],
  ["postflop", "Postflop"],
] as const;

export function TrainerTabs() {
  const [tab, setTab] = useState<(typeof TABS)[number][0]>("push-fold");
  return (
    <div className="flex flex-col gap-4">
      <div role="tablist" aria-label="Drills" className="flex gap-1">
        {TABS.map(([k, label]) => (
          <button
            key={k}
            role="tab"
            type="button"
            aria-selected={tab === k}
            onClick={() => setTab(k)}
            className={cn(
              "rounded-md px-3 py-1 text-sm",
              tab === k ? "bg-surface-3 text-fg" : "text-muted hover:bg-surface-2",
            )}
          >
            {label}
          </button>
        ))}
      </div>
      {tab === "push-fold" ? <PushFoldTrainer /> : tab === "preflop" ? <PreflopTrainer /> : <PostflopTrainer />}
    </div>
  );
}

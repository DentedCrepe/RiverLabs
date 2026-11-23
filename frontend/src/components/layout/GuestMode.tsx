"use client";

import { UserRound } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { setGuest, useGuest } from "@/lib/guest";

/** Switches guest mode, then reloads so every page shows the right games and progress. */
function toggle(on: boolean, onError: (msg: string) => void) {
  if (!setGuest(on)) {
    onError("This browser would not save the setting, so guest mode cannot be switched.");
    return;
  }
  window.location.reload();
}

export function GuestToggle() {
  const guest = useGuest();
  const [error, setError] = useState<string | null>(null);
  if (guest) return null; // the banner offers the way out
  return (
    <>
      <Button
        size="sm"
        variant="ghost"
        icon={<UserRound className="size-3.5" aria-hidden />}
        title="Let someone else train without changing your statistics"
        onClick={() => toggle(true, setError)}
      >
        Guest mode
      </Button>
      {error && (
        <span role="alert" className="text-xs text-danger">
          {error}
        </span>
      )}
    </>
  );
}

export function GuestBanner() {
  const guest = useGuest();
  const [error, setError] = useState<string | null>(null);
  if (!guest) return null;
  return (
    <div
      role="status"
      aria-label="Guest mode"
      className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-warn/40 bg-warn-dim px-4 py-2 text-xs text-fg lg:px-8"
    >
      <span className="font-semibold text-warn">Guest mode</span>
      <span className="min-w-0 flex-1">
        Answers are graded and games are played, but nothing counts towards the owner&apos;s progress, streak, goals,
        analytics or leaks. Goals, bookmarks and batch reviews are locked. Pages show the owner&apos;s progress; the
        game lists show guest games only.
      </span>
      <Button size="sm" onClick={() => toggle(false, setError)}>
        End guest mode
      </Button>
      {error && (
        <span role="alert" className="text-danger">
          {error}
        </span>
      )}
    </div>
  );
}

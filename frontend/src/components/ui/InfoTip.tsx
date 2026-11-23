"use client";

import { Info } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { cn } from "./cn";

/**
 * A small ⓘ that explains a term. Opens on hover (mouse) or tap (touch screens have no hover),
 * closes on a second tap, outside tap or Escape.
 */
export function InfoTip({
  label,
  children,
  className,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const [pinned, setPinned] = useState(false);
  const ref = useRef<HTMLSpanElement>(null);
  const id = useId();
  const shown = open || pinned;

  useEffect(() => {
    if (!pinned) return;
    const onDown = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setPinned(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setPinned(false);
    document.addEventListener("pointerdown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [pinned]);

  return (
    <span ref={ref} className={cn("relative inline-flex align-middle", className)}>
      <button
        type="button"
        aria-label={`What is ${label}?`}
        aria-expanded={shown}
        aria-describedby={shown ? id : undefined}
        onClick={() => setPinned((p) => !p)}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        className="inline-flex size-4 items-center justify-center rounded-full text-subtle hover:text-accent focus-visible:text-accent"
      >
        <Info aria-hidden className="size-3.5" />
      </button>
      {shown && (
        <span
          id={id}
          role="tooltip"
          className="absolute top-5 left-1/2 z-40 w-64 max-w-[80vw] -translate-x-1/2 rounded-md border border-line-strong bg-surface-3 px-3 py-2 text-left text-xs leading-relaxed font-normal tracking-normal text-fg normal-case shadow-lg"
        >
          {children}
        </span>
      )}
    </span>
  );
}

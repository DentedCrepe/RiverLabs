"use client";

import { type KeyboardEvent, useCallback, useEffect, useRef, useState } from "react";
import { GRID, type GridCell } from "@/lib/poker/grid";
import { cn } from "@/components/ui/cn";

export interface RangeMatrixProps {
  /** Weight in [0, 1] per hand-class label (e.g. "AKs"); missing = 0. */
  weights: Readonly<Record<string, number>>;
  /** Called with the label and new weight when the user paints a cell. */
  onPaint?: (label: string, weight: number) => void;
  /** Weight applied by a click / drag. */
  brush?: number;
  /** Optional per-cell secondary text (e.g. live combos). */
  annotate?: (cell: GridCell) => string | undefined;
  readOnly?: boolean;
}

const KIND_BG = { pair: "bg-surface-3", suited: "bg-surface-2", offsuit: "bg-surface" } as const;

/**
 * Interactive 13×13 hand-class matrix. Click or drag to paint the brush weight; painting a
 * cell that already has the brush weight erases it. Arrow keys move focus; Enter/Space paints.
 */
export function RangeMatrix({ weights, onPaint, brush = 1, annotate, readOnly = false }: RangeMatrixProps) {
  const [focus, setFocus] = useState(0);
  const painting = useRef<number | null>(null);
  const refs = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    const stop = () => {
      painting.current = null;
    };
    window.addEventListener("pointerup", stop);
    return () => window.removeEventListener("pointerup", stop);
  }, []);

  const paint = useCallback(
    (cell: GridCell, value: number) => {
      if (!readOnly && onPaint && (weights[cell.label] ?? 0) !== value) onPaint(cell.label, value);
    },
    [onPaint, readOnly, weights],
  );

  const toggleValue = (cell: GridCell) => ((weights[cell.label] ?? 0) === brush ? 0 : brush);

  const onKeyDown = (e: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const row = Math.floor(index / 13);
    const col = index % 13;
    const moves: Record<string, [number, number]> = {
      ArrowUp: [Math.max(0, row - 1), col],
      ArrowDown: [Math.min(12, row + 1), col],
      ArrowLeft: [row, Math.max(0, col - 1)],
      ArrowRight: [row, Math.min(12, col + 1)],
      Home: [row, 0],
      End: [row, 12],
    };
    const target = moves[e.key];
    if (target) {
      e.preventDefault();
      const next = target[0] * 13 + target[1];
      setFocus(next);
      refs.current[next]?.focus();
    }
  };

  return (
    <div
      role="group"
      aria-label="Starting hand matrix (arrow keys to move, Enter to paint)"
      className="grid touch-none gap-px rounded-md border border-line bg-line p-px select-none"
      style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }}
    >
      {GRID.map((cell, i) => {
        const w = weights[cell.label] ?? 0;
        const note = annotate?.(cell);
        return (
          <div key={cell.label} className="relative aspect-square min-w-0">
            <button
              ref={(el) => {
                refs.current[i] = el;
              }}
              type="button"
              tabIndex={i === focus ? 0 : -1}
              aria-label={`${cell.label}, ${Math.round(w * 100)}% weight${note ? `, ${note}` : ""}`}
              aria-pressed={w > 0}
              disabled={readOnly}
              onFocus={() => setFocus(i)}
              onKeyDown={(e) => onKeyDown(e, i)}
              onPointerDown={(e) => {
                e.preventDefault();
                const value = toggleValue(cell);
                painting.current = value;
                paint(cell, value);
              }}
              onPointerEnter={() => {
                if (painting.current !== null) paint(cell, painting.current);
              }}
              onClick={(e) => {
                // Keyboard activation (pointer clicks are handled on pointerdown).
                if (e.detail === 0) paint(cell, toggleValue(cell));
              }}
              className={cn(
                "group absolute inset-0 overflow-hidden text-left",
                KIND_BG[cell.kind],
                !readOnly && "cursor-pointer hover:brightness-125",
              )}
            >
              <span
                aria-hidden
                className="absolute inset-x-0 bottom-0 bg-accent/80 transition-[height]"
                style={{ height: `${w * 100}%` }}
              />
              <span
                className={cn(
                  "relative block px-[3px] pt-[2px] font-mono text-[clamp(7px,1.15vw,12px)] leading-none font-medium",
                  w > 0.5 ? "text-canvas" : "text-muted",
                )}
              >
                {cell.label}
              </span>
              {note && (
                <span className="absolute right-[3px] bottom-[2px] hidden font-mono text-[9px] text-subtle sm:block">
                  {note}
                </span>
              )}
            </button>
          </div>
        );
      })}
    </div>
  );
}

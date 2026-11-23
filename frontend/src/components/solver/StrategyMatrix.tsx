import { GRID } from "@/lib/poker/grid";
import type { ClassStrategy } from "@/lib/types";
import { cn } from "@/components/ui/cn";
import { actionColors } from "./actionColors";

/**
 * 13×13 view of one decision's strategy. Each cell's bar shows the weighted frequency of every
 * action for that hand class; classes outside the acting player's range are dimmed.
 */
export function StrategyMatrix({ actions, byClass }: { actions: string[]; byClass: ClassStrategy[] }) {
  const colors = actionColors(actions);
  const lookup = new Map(byClass.map((c) => [c.label, c]));
  const totalWeight = byClass.reduce((s, c) => s + c.weight, 0);
  const overall = actions.map(
    (_, a) => byClass.reduce((s, c) => s + c.weight * c.frequencies[a], 0) / (totalWeight || 1),
  );
  return (
    <div className="flex flex-col gap-3">
      <ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs" aria-label="Overall action frequencies">
        {actions.map((a, i) => (
          <li key={a} className="flex items-center gap-1.5">
            <span aria-hidden className="size-2.5 rounded-sm" style={{ background: colors[i] }} />
            <span className="text-fg">{a}</span>
            <span className="font-mono text-muted tabular">{(overall[i] * 100).toFixed(1)}%</span>
          </li>
        ))}
      </ul>
      <div
        role="img"
        aria-label="Strategy matrix by hand class"
        className="grid gap-px rounded-md border border-line bg-line p-px"
        style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }}
      >
        {GRID.map((cell) => {
          const c = lookup.get(cell.label);
          const label = c
            ? `${cell.label}: ${actions.map((a, i) => `${a} ${(c.frequencies[i] * 100).toFixed(0)}%`).join(", ")}`
            : `${cell.label}: not in range`;
          return (
            <div
              key={cell.label}
              title={label}
              aria-label={label}
              data-testid="strategy-cell"
              className={cn(
                "relative flex aspect-square min-w-0 flex-col justify-end overflow-hidden bg-surface",
                !c && "opacity-30",
              )}
            >
              {c && (
                <div className="absolute inset-0 flex" aria-hidden>
                  {c.frequencies.map((f, i) => (
                    <div key={i} style={{ width: `${f * 100}%`, background: colors[i] }} />
                  ))}
                </div>
              )}
              <span
                className={cn(
                  "relative mb-auto px-[3px] pt-[2px] font-mono text-[clamp(7px,1.1vw,11px)] leading-none font-medium",
                  c ? "text-canvas" : "text-muted",
                )}
              >
                {cell.label}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

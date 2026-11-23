"use client";

import { ChevronRight } from "lucide-react";
import { useState } from "react";
import { type HandRecord, type HeroDecision, type RecordAction, amount, describeAction } from "@/lib/play";
import { useApi } from "@/lib/useApi";
import { CardRow } from "@/components/poker/CardText";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

const STREETS = ["preflop", "flop", "turn", "river"] as const;
const BOARD_LEN: Record<string, number> = { preflop: 0, flop: 3, turn: 4, river: 5 };

const VERDICT_STYLE: Record<string, string> = {
  mistake: "bg-danger-dim text-danger",
  rare: "bg-warn-dim text-warn",
  ok: "bg-accent-dim text-accent-strong",
  not_graded: "bg-surface-3 text-muted",
};
const VERDICT_LABEL: Record<string, string> = {
  mistake: "Flagged",
  rare: "Unusual",
  ok: "Fine",
  not_graded: "Not graded",
};

function units(h: HandRecord): "bb" | "chips" {
  return h.tournament ? "chips" : "bb";
}

/** The model's frequencies for your hand at this spot, with your choice highlighted. */
function ModelBars({ model }: { model: NonNullable<HeroDecision["model"]> }) {
  return (
    <div className="flex flex-col gap-1" aria-label="Model frequencies for your hand">
      <p className="text-[11px] text-muted">
        Model with {model.class} ({model.spot.replaceAll("_", " ")}):
      </p>
      {model.actions.map((a, i) => (
        <div key={a} className="flex items-center gap-2 text-xs">
          <span className={cn("w-24 shrink-0 truncate", model.taken === i ? "font-semibold text-fg" : "text-muted")}>
            {a}
            {model.taken === i && " ← you"}
          </span>
          <span className="h-2 flex-1 overflow-hidden rounded-full bg-surface-3">
            <span
              className={cn("block h-full", model.taken === i ? "bg-accent" : "bg-line-strong")}
              style={{ width: `${model.frequencies[i] * 100}%` }}
            />
          </span>
          <span className="w-10 text-right font-mono text-muted tabular">
            {(model.frequencies[i] * 100).toFixed(0)}%
          </span>
        </div>
      ))}
    </div>
  );
}

function DecisionView({ d, h }: { d: HeroDecision; h: HandRecord }) {
  const f = d.feedback;
  return (
    <div className="ml-3 flex flex-col gap-2 rounded-md border border-line bg-surface-2/50 px-3 py-2">
      <div className="flex flex-wrap items-center gap-2 text-xs">
        {f && (
          <span className={cn("rounded px-1.5 py-px font-semibold uppercase", VERDICT_STYLE[f.verdict])}>
            {VERDICT_LABEL[f.verdict]}
          </span>
        )}
        {f && f.verdict !== "not_graded" && <span className="font-medium text-fg">{f.headline}</span>}
      </div>
      <p className="font-mono text-[11px] text-muted tabular">
        Pot {amount(d.pot, units(h))}
        {d.to_call > 0 && ` · to call ${amount(d.to_call, units(h))}`}
        {d.equity_vs_ranges !== null && ` · your equity ${(d.equity_vs_ranges * 100).toFixed(0)}%`}
        {d.required_equity !== null && ` · price needs ${(d.required_equity * 100).toFixed(0)}%`}
      </p>
      {d.model && <ModelBars model={d.model} />}
      {f && <p className="text-xs text-muted">{f.detail}</p>}
    </div>
  );
}

/** Street-by-street replay of one hand, with your decisions compared with the references. */
export function HandReview({ hand: h }: { hand: HandRecord }) {
  const u = units(h);
  const hero = h.seats.findIndex((s) => s.is_hero);
  // Your decisions are stored in the order you acted.
  let next = 0;
  const decisionFor = new Map<RecordAction, HeroDecision>();
  for (const a of h.actions)
    if (a.seat === hero && h.hero_decisions[next]) decisionFor.set(a, h.hero_decisions[next++]);
  const streets = STREETS.filter((st) => st === "preflop" || h.board.length >= BOARD_LEN[st]);
  return (
    <div className="flex flex-col gap-3">
      <ul className="grid grid-cols-1 gap-1.5 text-xs sm:grid-cols-2" aria-label="Players">
        {h.seats.map((s, i) => (
          <li
            key={s.name}
            className={cn(
              "flex items-center justify-between gap-2 rounded border px-2 py-1",
              s.is_hero ? "border-accent/40" : "border-line",
            )}
          >
            <span className="truncate">
              <span className="text-muted">{s.position}</span> {s.name}
            </span>
            <span className="flex items-center gap-1.5">
              <span
                className={cn(!s.is_hero && !s.shown && "opacity-60")}
                title={!s.is_hero && !s.shown ? "Not shown at the table" : undefined}
              >
                <CardRow cards={s.hole_cards} size="sm" />
              </span>
              <span className={cn("font-mono tabular", h.net[i] >= 0 ? "text-accent-strong" : "text-danger")}>
                {h.net[i] >= 0 ? "+" : "−"}
                {amount(Math.abs(h.net[i]), u)}
              </span>
            </span>
          </li>
        ))}
      </ul>
      {streets.map((st) => {
        const acts = h.actions.filter((a) => a.street === st);
        return (
          <section key={st} aria-label={st} className="flex flex-col gap-1.5">
            <h4 className="flex items-center gap-2 text-xs font-semibold text-fg capitalize">
              {st}
              {BOARD_LEN[st] > 0 && <CardRow cards={h.board.slice(0, BOARD_LEN[st])} size="sm" />}
            </h4>
            {acts.map((a, i) => {
              const d = decisionFor.get(a);
              return (
                <div key={i} className="flex flex-col gap-1">
                  <p className={cn("text-xs", a.seat === hero ? "font-medium text-accent-strong" : "text-muted")}>
                    {a.seat === hero ? "You" : h.seats[a.seat].name}{" "}
                    {describeAction(a.action, u, a.all_in, a.seat === hero)}
                  </p>
                  {d && <DecisionView d={d} h={h} />}
                </div>
              );
            })}
          </section>
        );
      })}
      {h.summary.length > 0 && (
        <ul className="text-sm text-fg">
          {h.summary.map((l) => (
            <li key={l}>{l}</li>
          ))}
        </ul>
      )}
      {h.revealed && <p className="text-[11px] text-warn">Played with the bots&apos; cards revealed.</p>}
    </div>
  );
}

/** Every hand of the session, newest first; tap one to review it. */
export function HandHistory({ sessionId, refreshKey }: { sessionId: string; refreshKey: number }) {
  const { data, error, loading } = useApi<HandRecord[]>(`/api/v1/play/sessions/${sessionId}/hands?v=${refreshKey}`);
  const [open, setOpen] = useState<number | null>(null);
  if (error) return <ErrorState message={error.message} />;
  if (loading && !data) return <LoadingState label="Loading hands…" />;
  if (!data || data.length === 0) return <EmptyState title="No hands yet" description="Finished hands appear here." />;
  return (
    <ul className="divide-y divide-line" aria-label="Hand history">
      {[...data].reverse().map((h) => {
        const hero = h.seats.findIndex((s) => s.is_hero);
        const flagged = h.hero_decisions.filter((d) => d.feedback?.verdict === "mistake").length;
        const unusual = h.hero_decisions.filter((d) => d.feedback?.verdict === "rare").length;
        const isOpen = open === h.hand_no;
        return (
          <li key={h.hand_no} className="py-1.5">
            <button
              type="button"
              aria-expanded={isOpen}
              onClick={() => setOpen(isOpen ? null : h.hand_no)}
              className="flex w-full flex-wrap items-center gap-x-3 gap-y-1 rounded px-1 py-1 text-left text-xs hover:bg-surface-2"
            >
              <ChevronRight
                aria-hidden
                className={cn("size-3.5 shrink-0 text-subtle transition-transform", isOpen && "rotate-90")}
              />
              <span className="w-8 font-mono text-muted tabular">#{h.hand_no}</span>
              <span className="w-8 text-muted">{h.seats[hero].position}</span>
              <CardRow cards={h.seats[hero].hole_cards} size="sm" />
              {h.board.length > 0 && <CardRow cards={h.board} size="sm" />}
              <span
                className={cn("ml-auto font-mono tabular", h.net[hero] >= 0 ? "text-accent-strong" : "text-danger")}
              >
                {h.net[hero] >= 0 ? "+" : "−"}
                {amount(Math.abs(h.net[hero]), units(h))}
              </span>
              {flagged > 0 && (
                <span className={cn("rounded px-1.5 py-px", VERDICT_STYLE.mistake)}>{flagged} flagged</span>
              )}
              {unusual > 0 && <span className={cn("rounded px-1.5 py-px", VERDICT_STYLE.rare)}>{unusual} unusual</span>}
            </button>
            {isOpen && (
              <div className="mt-2 pb-2 pl-6">
                <HandReview hand={h} />
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}

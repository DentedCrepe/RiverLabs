"use client";

import { ChevronRight, Cpu } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import {
  type DecisionReview,
  type HandRecord,
  type HeroDecision,
  type RecordAction,
  amount,
  describeAction,
} from "@/lib/play";
import { SolverDecision, SolverReviewPanel, useSolverReview } from "./SolverReview";
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

function DecisionView({
  d,
  h,
  solveHref,
  solver,
}: {
  d: HeroDecision;
  h: HandRecord;
  solveHref?: string;
  solver?: DecisionReview;
}) {
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
      {solver && <SolverDecision r={solver} />}
      {solveHref && d.spot && (
        <Link
          href={solveHref}
          className="inline-flex w-fit items-center gap-1.5 rounded-md border border-line-strong px-2 py-1 text-xs font-medium text-accent hover:bg-surface-2"
        >
          <Cpu aria-hidden className="size-3.5" />
          Solve this spot
        </Link>
      )}
    </div>
  );
}

/** Street-by-street replay of one hand, with your decisions compared with the references. */
export function HandReview({ hand: h, sessionId }: { hand: HandRecord; sessionId?: string }) {
  const u = units(h);
  const hero = h.seats.findIndex((s) => s.is_hero);
  // Your decisions are stored in the order you acted.
  let next = 0;
  const decisionFor = new Map<RecordAction, number>();
  for (const a of h.actions) if (a.seat === hero && h.hero_decisions[next]) decisionFor.set(a, next++);
  const streets = STREETS.filter((st) => st === "preflop" || h.board.length >= BOARD_LEN[st]);
  const sr = useSolverReview(sessionId, h.hand_no);
  const solverFor = new Map<number, DecisionReview>();
  for (const st of sr.review?.streets ?? []) for (const r of st.review?.decisions ?? []) solverFor.set(r.decision, r);
  const reviewable = Boolean(sessionId) && h.hero_decisions.some((d) => d.spot);
  return (
    <div className="flex flex-col gap-3">
      {reviewable && (
        <SolverReviewPanel review={sr.review} error={sr.error} starting={sr.starting} onStart={sr.start} />
      )}
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
              const di = decisionFor.get(a);
              const d = di === undefined ? undefined : h.hero_decisions[di];
              const solveHref =
                sessionId && di !== undefined
                  ? `/solver?game=${sessionId}&hand=${h.hand_no}&decision=${di}`
                  : undefined;
              return (
                <div key={i} className="flex flex-col gap-1">
                  <p className={cn("text-xs", a.seat === hero ? "font-medium text-accent-strong" : "text-muted")}>
                    {a.seat === hero ? "You" : h.seats[a.seat].name}{" "}
                    {describeAction(a.action, u, a.all_in, a.seat === hero)}
                  </p>
                  {d && (
                    <DecisionView
                      d={d}
                      h={h}
                      solveHref={solveHref}
                      solver={di === undefined ? undefined : solverFor.get(di)}
                    />
                  )}
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

/** One hand of a list (a session's hands, or hands across sessions). */
export type HandItem = { sessionId: string; hand: HandRecord; label?: string };

/** Hands as rows; tap one to replay it street by street. */
export function HandRows({ items, ariaLabel }: { items: HandItem[]; ariaLabel: string }) {
  const [open, setOpen] = useState<string | null>(null);
  return (
    <ul className="divide-y divide-line" aria-label={ariaLabel}>
      {items.map(({ sessionId, hand: h, label }) => {
        const key = `${sessionId}:${h.hand_no}`;
        const hero = h.seats.findIndex((s) => s.is_hero);
        const flagged = h.hero_decisions.filter((d) => d.feedback?.verdict === "mistake").length;
        const unusual = h.hero_decisions.filter((d) => d.feedback?.verdict === "rare").length;
        const isOpen = open === key;
        return (
          <li key={key} className="py-1.5">
            <button
              type="button"
              aria-expanded={isOpen}
              onClick={() => setOpen(isOpen ? null : key)}
              className="flex w-full flex-wrap items-center gap-x-3 gap-y-1 rounded px-1 py-1 text-left text-xs hover:bg-surface-2"
            >
              <ChevronRight
                aria-hidden
                className={cn("size-3.5 shrink-0 text-subtle transition-transform", isOpen && "rotate-90")}
              />
              {label && <span className="text-subtle">{label}</span>}
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
                <HandReview hand={h} sessionId={sessionId} />
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}

/** Every hand of the session, newest first; tap one to review it. */
export function HandHistory({ sessionId, refreshKey }: { sessionId: string; refreshKey: number }) {
  const { data, error, loading } = useApi<HandRecord[]>(`/api/v1/play/sessions/${sessionId}/hands?v=${refreshKey}`);
  if (error) return <ErrorState message={error.message} />;
  if (loading && !data) return <LoadingState label="Loading hands…" />;
  if (!data || data.length === 0) return <EmptyState title="No hands yet" description="Finished hands appear here." />;
  return <HandRows ariaLabel="Hand history" items={[...data].reverse().map((hand) => ({ sessionId, hand }))} />;
}

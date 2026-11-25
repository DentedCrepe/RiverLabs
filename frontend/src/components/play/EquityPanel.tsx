"use client";

import { Calculator, ChevronDown, ChevronUp } from "lucide-react";
import { useState } from "react";
import { ApiError, api } from "@/lib/api";
import { useEquityMode } from "@/lib/equityMode";
import { GLOSSARY } from "@/lib/glossary";
import { type EquityBreakdown, HAND_CATEGORY, type OpponentRange, type TableView, rangeClasses } from "@/lib/play";
import { CardRow } from "@/components/poker/CardText";
import { StrategyMatrix } from "@/components/solver/StrategyMatrix";
import { Button } from "@/components/ui/Button";
import { InfoTip } from "@/components/ui/InfoTip";
import { cn } from "@/components/ui/cn";

const pct = (x: number) => `${(x * 100).toFixed(x > 0 && x < 0.01 ? 1 : 0)}%`;

const STEP_SOURCE: Record<string, string> = {
  preflop_model: "following GTOpen's preflop model",
  preflop_rules: "by its preflop rules",
  postflop_rules: "by its postflop rules",
};

/** Your equity against the bots' ranges, and the price you are getting. */
export function OddsStrip({ equity, required }: { equity: number; required: number | null }) {
  const enough = required === null || equity >= required;
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm" aria-label="Your odds">
      <span className="flex items-center gap-1 text-muted">
        Your equity vs their ranges
        <InfoTip label="your equity">{GLOSSARY.heroEquity}</InfoTip>
      </span>
      <span className={cn("font-mono font-semibold tabular", enough ? "text-accent-strong" : "text-danger")}>
        {(equity * 100).toFixed(0)}%
      </span>
      {required !== null && (
        <span className="text-muted">
          Price needs <span className="font-mono text-fg tabular">{(required * 100).toFixed(0)}%</span>
        </span>
      )}
      <span className="h-1.5 w-full overflow-hidden rounded-full bg-surface-3" aria-hidden>
        <span
          className={cn("block h-full", enough ? "bg-accent" : "bg-danger")}
          style={{ width: `${equity * 100}%` }}
        />
      </span>
    </div>
  );
}

function OpponentCard({ o, board }: { o: OpponentRange; board: string[] }) {
  const classes = rangeClasses(o);
  return (
    <section
      className="flex flex-col gap-2 rounded-md border border-line bg-surface-2/40 p-3"
      aria-label={`${o.name}'s range`}
    >
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h4 className="text-sm font-semibold text-fg">
          {o.name} <span className="font-normal text-muted">({o.position})</span>
        </h4>
        {o.hero_equity !== null && (
          <span className="text-xs text-muted">
            You have <span className="font-mono text-fg tabular">{pct(o.hero_equity)}</span> against this range alone
          </span>
        )}
      </header>
      <p className="text-xs text-muted">
        About <span className="font-mono text-fg tabular">{o.combos.toFixed(0)}</span> hand combinations,{" "}
        <span className="font-mono text-fg tabular">{pct(o.share_of_all)}</span> of all the hands they could hold.
      </p>
      <ol className="flex flex-col gap-1 text-xs" aria-label="How their actions narrowed the range">
        {o.steps.length === 0 ? (
          <li className="text-muted">They have not acted yet this hand: any hand is still possible.</li>
        ) : (
          o.steps.map((s, i) => (
            <li key={i} className="flex flex-wrap gap-x-1.5">
              <span className="capitalize text-muted">{s.street}:</span>
              <span className="text-fg">{s.action}</span>
              <span className="text-muted">
                ({STEP_SOURCE[s.source]}) kept <span className="font-mono text-fg tabular">{pct(s.kept)}</span> of the
                hands that could still have done it
              </span>
            </li>
          ))
        )}
      </ol>
      {board.length >= 3 && o.categories.length > 0 && (
        <table className="w-full text-xs" aria-label="What their range makes on this board">
          <thead>
            <tr className="text-left text-muted">
              <th className="pb-1 font-normal">On this board they have</th>
              <th className="pb-1 text-right font-normal">Share</th>
              <th className="pb-1 text-right font-normal">Your equity</th>
            </tr>
          </thead>
          <tbody>
            {o.categories.map((c) => (
              <tr key={c.category} className="border-t border-line/60">
                <td className="py-0.5 text-fg">{HAND_CATEGORY[c.category] ?? c.category}</td>
                <td className="py-0.5 text-right font-mono tabular">{pct(c.share)}</td>
                <td
                  className={cn(
                    "py-0.5 text-right font-mono tabular",
                    c.hero_equity >= 0.5 ? "text-accent-strong" : "text-danger",
                  )}
                >
                  {pct(c.hero_equity)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {o.held && (
        <p className="flex flex-wrap items-center gap-1.5 text-xs text-fg" aria-label={`What ${o.name} held`}>
          They actually held <CardRow cards={o.held.cards} size="sm" />
          <span className="text-muted">
            — their range kept <span className="font-mono text-fg tabular">{pct(o.held.weight)}</span> of that exact
            hand (the share of the time it would have played this way)
          </span>
        </p>
      )}
      {classes.length > 0 && (
        <details>
          <summary className="cursor-pointer text-xs text-accent">Their range, hand by hand</summary>
          <div className="mt-2">
            <StrategyMatrix actions={["still in range"]} byClass={classes} />
          </div>
        </details>
      )}
    </section>
  );
}

/** What an equity figure was made of: your hand, then each opponent's tracked range. */
export function Why({ b }: { b: EquityBreakdown }) {
  return (
    <div className="flex flex-col gap-3" aria-label="Why this equity">
      <p className="flex flex-wrap items-center gap-2 text-sm text-fg">
        You hold <CardRow cards={b.hero_cards} size="sm" />
        {b.hero_hand && <span>: {HAND_CATEGORY[b.hero_hand]?.toLowerCase() ?? b.hero_hand}</span>}
        {b.board.length > 0 && (
          <span className="flex items-center gap-1 text-muted">
            on <CardRow cards={b.board} size="sm" />
          </span>
        )}
      </p>
      <p className="text-xs text-muted">
        Each bot&apos;s range is exact for that bot: after every action it took, each hand was weighted by how often its
        own strategy makes that move with it. Hands using your cards or the board are left out.{" "}
        {b.method === "exact"
          ? "Your equity is an exact count over every remaining card."
          : `Your equity is estimated from ${(b.samples ?? 0).toLocaleString()} random deals (Monte Carlo).`}
      </p>
      {b.opponents.map((o) => (
        <OpponentCard key={o.player} o={o} board={b.board} />
      ))}
      {b.hero_range && (
        <details>
          <summary className="cursor-pointer text-xs text-accent">
            Your range, as the bots&apos; preflop model sees it
          </summary>
          <p className="mt-1 font-mono text-[11px] break-words text-muted">{b.hero_range}</p>
          <p className="text-[11px] text-subtle">
            The hands you would play this way in GTOpen&apos;s preflop model (an approximate model), up to the flop.
          </p>
        </details>
      )}
    </div>
  );
}

/**
 * Your equity at this decision: hidden until you ask (the default), always shown, or off (a
 * setting on the table). "Why this number?" shows each bot's range and how it got there.
 */
export function EquityPanel({ sessionId, t }: { sessionId: string; t: TableView }) {
  const mode = useEquityMode();
  const [b, setB] = useState<EquityBreakdown | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (mode === "off") return null;

  async function load(expand: boolean) {
    setBusy(true);
    setError(null);
    try {
      setB(await api.get<EquityBreakdown>(`/api/v1/play/sessions/${sessionId}/equity`));
      setOpen(expand);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  const equity = b?.equity ?? (mode === "always" ? t.hero_equity : null);
  const required = b?.required ?? t.required_equity;
  return (
    <div className="flex flex-col gap-2">
      {equity !== null && equity !== undefined ? (
        <OddsStrip equity={equity} required={required} />
      ) : (
        <div>
          <Button
            size="sm"
            variant="ghost"
            loading={busy}
            onClick={() => load(false)}
            icon={<Calculator className="size-3.5" />}
          >
            Show my equity
          </Button>
        </div>
      )}
      {equity !== null && equity !== undefined && (
        <div>
          <Button
            size="sm"
            variant="ghost"
            loading={busy && b === null}
            onClick={() => (b ? setOpen(!open) : load(true))}
            icon={open ? <ChevronUp className="size-3.5" /> : <ChevronDown className="size-3.5" />}
          >
            {open ? "Hide why" : "Why this number?"}
          </Button>
        </div>
      )}
      {open && b && (
        <div className="max-h-[60vh] overflow-y-auto rounded-md border border-line bg-canvas p-3">
          <Why b={b} />
        </div>
      )}
      {error && (
        <p role="alert" className="text-xs text-danger">
          {error}
        </p>
      )}
    </div>
  );
}

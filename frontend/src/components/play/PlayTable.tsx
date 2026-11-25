"use client";

import { ChevronDown, DoorOpen, Eye, EyeOff, Play } from "lucide-react";
import { useEffect, useState } from "react";
import { ApiError, api } from "@/lib/api";
import { type EquityMode, setEquityMode, useEquityMode } from "@/lib/equityMode";
import { GLOSSARY } from "@/lib/glossary";
import { type Feedback, type PlayState, type TableView, amount, parseAmount } from "@/lib/play";
import { Button } from "@/components/ui/Button";
import { TextInput } from "@/components/ui/Field";
import { InfoTip } from "@/components/ui/InfoTip";
import { ErrorState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { EquityPanel } from "./EquityPanel";
import { TableFelt } from "./TableFelt";

/** Your live equity against the bots' ranges, and the price when facing a bet. */
const VERDICT: Record<Feedback["verdict"], { label: string; className: string }> = {
  mistake: { label: "Flagged", className: "border-danger/50 bg-danger-dim/40" },
  rare: { label: "Unusual", className: "border-warn/50 bg-warn-dim/40" },
  ok: { label: "Fine", className: "border-accent/40 bg-accent-dim/30" },
  not_graded: { label: "Not graded", className: "border-line bg-surface-2/60" },
};

function FeedbackCard({ item, compact }: { item: TableView["feedback"][number]; compact?: boolean }) {
  const f = item.feedback;
  const v = VERDICT[f.verdict];
  return (
    <div className={cn("rounded-md border px-3 py-2", v.className)} aria-label={`Feedback: ${v.label}`}>
      <p className="flex flex-wrap items-center gap-x-2 text-sm">
        <span className="text-[10px] font-semibold tracking-wide uppercase">{v.label}</span>
        {f.verdict !== "not_graded" && <span className="font-medium text-fg">{f.headline}</span>}
        <span className="text-xs text-muted">
          · {item.street}: you {item.action}
        </span>
      </p>
      {!compact && <p className="mt-0.5 text-xs text-muted">{f.detail}</p>}
      {!compact && f.reference && <p className="mt-0.5 text-[11px] text-subtle">Compared with: {f.reference}</p>}
    </div>
  );
}

function Controls({
  t,
  sessionId,
  onAct,
  busy,
}: {
  t: TableView;
  sessionId: string;
  onAct: (a: Record<string, unknown>) => void;
  busy: boolean;
}) {
  const l = t.legal!;
  const range = l.bet ?? l.raise;
  const kind = l.bet ? "bet" : "raise";
  const hero = t.seats.find((s) => s.is_hero)!;
  const [size, setSize] = useState(range ? range[0] : 0);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const clamp = (x: number) => (range ? Math.min(Math.max(Math.round(x), range[0]), range[1]) : x);
  // Pot-fraction sizes: raise to = (the bet you face) + fraction × (the pot after you call).
  // With nothing to call this is simply fraction × pot.
  const toCall = l.call ?? 0;
  const facing = hero.street_commit + toCall;
  const potAfterCall = t.pot + toCall;
  const presets = range
    ? (
        [
          ["Min", range[0]],
          ["½ pot", facing + potAfterCall / 2],
          ["¾ pot", facing + (potAfterCall * 3) / 4],
          ["Pot", facing + potAfterCall],
          ["All-in", range[1]],
        ] as const
      ).map(([label, to]) => [label, clamp(to)] as const)
    : [];
  // Slider steps of a tenth of a big blind (or one chip).
  const step = Math.max(1, Math.round(t.big_blind / 10));

  function submitTyped() {
    const to = parseAmount(text, t.units);
    if (to === null) return setError(t.units === "chips" ? "Whole chips" : "bb, up to 3 decimals");
    setError(null);
    onAct({ type: kind, to });
  }

  return (
    <div
      className="sticky bottom-0 z-20 -mx-4 flex flex-col gap-2 border-t border-line bg-canvas/95 px-4 py-3 backdrop-blur sm:mx-0 sm:rounded-md sm:border"
      aria-label="Your action"
    >
      <EquityPanel key={`${t.hand_no}-${t.log.length}`} sessionId={sessionId} t={t} />
      <div className="grid grid-cols-3 gap-2">
        {l.fold ? (
          <Button disabled={busy} onClick={() => onAct({ type: "fold" })} className="h-11 justify-center">
            Fold
          </Button>
        ) : (
          <span />
        )}
        {l.check ? (
          <Button disabled={busy} onClick={() => onAct({ type: "check" })} className="h-11 justify-center">
            Check
          </Button>
        ) : l.call !== null ? (
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => onAct({ type: "call" })}
            className="h-11 justify-center"
          >
            Call {amount(l.call, t.units)}
          </Button>
        ) : (
          <span />
        )}
        {range ? (
          <Button
            variant="primary"
            disabled={busy}
            onClick={() => onAct({ type: kind, to: size })}
            className="h-11 justify-center"
          >
            {kind === "bet" ? "Bet" : "Raise to"} {amount(size, t.units)}
          </Button>
        ) : (
          <span />
        )}
      </div>
      {range && (
        <>
          <div className="flex flex-wrap gap-1.5" role="group" aria-label="Bet sizes">
            {presets.map(([label, to]) => (
              <button
                key={label}
                type="button"
                onClick={() => setSize(to)}
                aria-pressed={size === to}
                className={cn(
                  "rounded-full border px-2.5 py-1 text-xs",
                  size === to ? "border-accent bg-accent-dim text-accent-strong" : "border-line text-muted",
                )}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="flex items-center gap-2">
            <input
              type="range"
              aria-label={`${kind === "bet" ? "Bet" : "Raise"} size`}
              min={range[0]}
              max={range[1]}
              step={step}
              value={size}
              onChange={(e) => setSize(clamp(Number(e.target.value)))}
              className="flex-1 accent-[var(--color-accent)]"
            />
            <form
              className="flex items-center gap-1"
              onSubmit={(e) => {
                e.preventDefault();
                submitTyped();
              }}
            >
              <TextInput
                aria-label={`Exact ${kind} to (${t.units})`}
                className="w-20"
                inputMode="decimal"
                placeholder={t.units === "chips" ? "chips" : "bb"}
                value={text}
                onChange={(e) => setText(e.target.value)}
              />
              <Button size="sm" type="submit" disabled={busy || !text}>
                Go
              </Button>
            </form>
          </div>
        </>
      )}
      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  );
}

/** The table for an active game. */
export function PlayTable({ state, onChange }: { state: PlayState; onChange: (s: PlayState) => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);
  const [autoDeal, setAutoDeal] = useState(false);
  const equityMode = useEquityMode();
  const t = state.table;
  const id = state.session.id;

  async function call(path: string, body?: unknown) {
    setBusy(true);
    setError(null);
    try {
      const next = await api.post<PlayState>(`/api/v1/play/sessions/${id}/${path}`, body ?? {});
      if (path === "next") setShowAll(false);
      onChange(next);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  // Auto-deal, pausing when the last hand had a flagged or unusual decision so you can read it.
  const pause = t?.feedback.some((f) => f.feedback.verdict === "mistake" || f.feedback.verdict === "rare");
  const handOver = t?.status === "hand_over";
  useEffect(() => {
    if (!autoDeal || !handOver || pause || busy) return;
    const timer = setTimeout(() => void call("next"), 1800);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `call` is stable enough for this timer
  }, [autoDeal, handOver, pause, busy, t?.hand_no]);

  if (!t) return <ErrorState message="This game is no longer in memory; its report is still available." />;
  const over = t.status === "session_over";
  const latest = t.feedback.at(-1);
  const earlier = t.feedback.slice(0, -1);
  const anyHidden = t.seats.some((s) => s.hidden_cards);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
        <span className="font-mono tabular">Hand {t.hand_no}</span>
        <span>
          Blinds {amount(t.small_blind, t.units)}/{amount(t.big_blind, t.units)}
          {t.ante > 0 && `, ante ${amount(t.ante, t.units)}`}
          {t.level !== null && ` · level ${t.level}`}
        </span>
        {t.payouts && t.payouts.length > 0 && (
          <span>
            Pays {t.payouts.join("/")}%<InfoTip label="ICM">{GLOSSARY.icm}</InfoTip>
          </span>
        )}
        <span className={cn("font-mono tabular", t.hero_total_net >= 0 ? "text-accent-strong" : "text-danger")}>
          Session {t.hero_total_net >= 0 ? "+" : "−"}
          {amount(Math.abs(t.hero_total_net), t.units)}
        </span>
        <span className="ml-auto flex items-center gap-1">
          <Button
            size="sm"
            variant="ghost"
            disabled={busy || over}
            aria-pressed={t.reveal}
            onClick={() => call("reveal", { on: !t.reveal })}
            icon={t.reveal ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
          >
            {t.reveal ? "Hide bots' cards" : "Reveal bots' cards"}
          </Button>
          <InfoTip label="revealing cards">{GLOSSARY.reveal}</InfoTip>
          <label className="flex items-center gap-1 text-xs text-muted">
            Equity
            <select
              aria-label="Show your equity"
              value={equityMode}
              onChange={(e) => setEquityMode(e.target.value as EquityMode)}
              className="h-7 rounded-md border border-line bg-canvas px-1.5 text-xs text-fg"
            >
              <option value="request">On request</option>
              <option value="always">Always</option>
              <option value="off">Off</option>
            </select>
          </label>
          <Button
            size="sm"
            variant="ghost"
            disabled={busy || over}
            onClick={() => call("leave")}
            icon={<DoorOpen className="size-3.5" />}
          >
            Leave
          </Button>
        </span>
      </div>

      <TableFelt t={t} showAll={showAll} />

      {t.status !== "awaiting_hero" && t.summary.length > 0 && (
        <ul className="flex flex-col gap-0.5 text-sm text-fg" aria-label="Hand result">
          {t.summary.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}

      {latest && (
        <section aria-label="Feedback on your decisions" className="flex flex-col gap-1.5">
          <p className="flex items-center gap-1 text-xs font-medium text-muted">
            Your last decision
            <InfoTip label="decision feedback">{GLOSSARY.feedback}</InfoTip>
          </p>
          <FeedbackCard item={latest} />
          {earlier.length > 0 && (
            <details className="text-xs">
              <summary className="cursor-pointer text-muted">Earlier this hand ({earlier.length})</summary>
              <div className="mt-1.5 flex flex-col gap-1.5">
                {earlier.map((f, i) => (
                  <FeedbackCard key={i} item={f} />
                ))}
              </div>
            </details>
          )}
        </section>
      )}

      {t.status === "hand_over" && (
        <div className="flex flex-wrap items-center gap-3">
          <Button
            variant="primary"
            loading={busy}
            onClick={() => call("next")}
            icon={<Play className="size-4" />}
            className="h-11"
          >
            {t.hand_no === 0 ? "Deal the first hand" : "Next hand"}
          </Button>
          {anyHidden && (
            <Button variant="ghost" onClick={() => setShowAll((v) => !v)}>
              {showAll ? "Hide unshown hands" : "Show all hands"}
            </Button>
          )}
          <label className="inline-flex items-center gap-2 text-xs text-muted">
            <input
              type="checkbox"
              className="size-4"
              checked={autoDeal}
              onChange={(e) => setAutoDeal(e.target.checked)}
            />
            Auto-deal (pauses after a flagged decision)
          </label>
        </div>
      )}
      {over && <p className="text-sm text-fg">The session is over. The report is below.</p>}
      {error && <ErrorState message={error} />}

      {t.log.length > 0 && (
        <details className="rounded-md border border-line px-3 py-2 text-xs">
          <summary className="flex cursor-pointer items-center gap-1 text-muted">
            <ChevronDown className="size-3.5" aria-hidden /> Hand log ({t.log.length} actions)
          </summary>
          <ol className="mt-1.5 max-h-48 overflow-y-auto" aria-label="Hand log">
            {t.log.map((l, i) => (
              <li key={i} className="flex gap-2">
                <span className="w-12 shrink-0 text-subtle capitalize">{l.street}</span>
                <span className="text-fg">{l.player}</span>
                <span className="text-muted">{l.text}</span>
              </li>
            ))}
          </ol>
        </details>
      )}
      {t.notes.map((n) => (
        <p key={n} className="text-xs text-subtle">
          {n}
        </p>
      ))}

      {t.status === "awaiting_hero" && t.legal && (
        <Controls key={t.log.length} t={t} sessionId={id} onAct={(a) => call("act", { action: a })} busy={busy} />
      )}
    </div>
  );
}

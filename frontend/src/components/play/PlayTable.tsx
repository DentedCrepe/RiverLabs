"use client";

import { DoorOpen, Play } from "lucide-react";
import { useState } from "react";
import { ApiError, api } from "@/lib/api";
import { type PlayState, type SeatView, type TableView, amount, parseAmount } from "@/lib/play";
import { Button } from "@/components/ui/Button";
import { TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";
import { CardRow } from "@/components/poker/CardText";
import { cn } from "@/components/ui/cn";

function Seat({ seat, units, toAct }: { seat: SeatView; units: TableView["units"]; toAct: boolean }) {
  return (
    <div
      data-testid="seat"
      className={cn(
        "flex min-w-0 flex-col gap-1 rounded-md border px-2.5 py-2 text-xs",
        toAct ? "border-accent bg-accent-dim/40" : "border-line bg-surface-2/60",
        (seat.folded || seat.busted) && "opacity-50",
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className={cn("truncate font-medium", seat.is_hero ? "text-accent-strong" : "text-fg")}>{seat.name}</span>
        {seat.is_button && (
          <span aria-label="Dealer button" className="rounded-full bg-fg px-1.5 text-[10px] font-bold text-canvas">
            D
          </span>
        )}
      </div>
      <div className="flex items-center justify-between gap-2 font-mono tabular text-muted">
        <span>{seat.position ?? (seat.busted ? `out${seat.finish ? ` (${seat.finish})` : ""}` : "—")}</span>
        <span>{amount(seat.stack, units)}</span>
      </div>
      <div className="flex min-h-6 items-center justify-between gap-2">
        {seat.cards ? (
          <CardRow cards={seat.cards} size="sm" />
        ) : (
          <span className="text-subtle">{seat.in_hand && !seat.folded ? "🂠🂠" : ""}</span>
        )}
        {seat.street_commit > 0 && (
          <span className="font-mono text-warn tabular">{amount(seat.street_commit, units)}</span>
        )}
        {seat.all_in && <span className="text-[10px] font-semibold text-danger uppercase">all-in</span>}
      </div>
    </div>
  );
}

function Controls({ t, onAct, busy }: { t: TableView; onAct: (a: Record<string, unknown>) => void; busy: boolean }) {
  const l = t.legal!;
  const range = l.bet ?? l.raise;
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const kind = l.bet ? "bet" : "raise";
  const hero = t.seats.find((s) => s.is_hero)!;
  const presets = range
    ? [
        ["½ pot", Math.round(t.pot / 2) + hero.street_commit],
        ["¾ pot", Math.round((t.pot * 3) / 4) + hero.street_commit],
        ["Pot", t.pot + (l.call ?? 0) + hero.street_commit],
      ].map(([label, to]) => [label, Math.min(Math.max(to as number, range[0]), range[1])] as const)
    : [];
  function raise(to: number) {
    setError(null);
    onAct({ type: kind, to });
  }
  function submitTyped() {
    if (!range) return;
    const to = parseAmount(text, t.units);
    if (to === null) {
      setError(t.units === "chips" ? "Whole chips" : "bb, up to 3 decimals");
      return;
    }
    raise(to);
  }
  return (
    <div className="flex flex-col gap-2" aria-label="Your action">
      <div className="flex flex-wrap gap-2">
        {l.fold && (
          <Button size="sm" disabled={busy} onClick={() => onAct({ type: "fold" })}>
            Fold
          </Button>
        )}
        {l.check && (
          <Button size="sm" disabled={busy} onClick={() => onAct({ type: "check" })}>
            Check
          </Button>
        )}
        {l.call !== null && (
          <Button size="sm" variant="primary" disabled={busy} onClick={() => onAct({ type: "call" })}>
            Call {amount(l.call, t.units)}
          </Button>
        )}
      </div>
      {range && (
        <div className="flex flex-wrap items-center gap-2">
          {presets.map(([label, to]) => (
            <Button key={label} size="sm" variant="ghost" disabled={busy} onClick={() => raise(to)}>
              {label} ({amount(to, t.units)})
            </Button>
          ))}
          <Button size="sm" variant="ghost" disabled={busy} onClick={() => raise(range[1])}>
            All-in ({amount(range[1], t.units)})
          </Button>
          <form
            className="flex items-center gap-1"
            onSubmit={(e) => {
              e.preventDefault();
              submitTyped();
            }}
          >
            <TextInput
              aria-label={`${kind === "bet" ? "Bet" : "Raise"} to (${t.units})`}
              className="w-24"
              placeholder={amount(range[0], t.units)}
              value={text}
              onChange={(e) => setText(e.target.value)}
            />
            <Button size="sm" type="submit" disabled={busy}>
              {kind === "bet" ? "Bet" : "Raise to"}
            </Button>
          </form>
        </div>
      )}
      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  );
}

/** The table for an active game. */
export function PlayTable({ state, onChange }: { state: PlayState; onChange: (s: PlayState) => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const t = state.table;
  const id = state.session.id;

  async function call(path: string, body?: unknown) {
    setBusy(true);
    setError(null);
    try {
      onChange(await api.post<PlayState>(`/api/v1/play/sessions/${id}/${path}`, body ?? {}));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!t) return <ErrorState message="This game is no longer in memory; its report is still available." />;
  const over = t.status === "session_over";
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3 text-xs text-muted">
        <span className="font-mono tabular">Hand {t.hand_no}</span>
        <span>
          Blinds {amount(t.small_blind, t.units)}/{amount(t.big_blind, t.units)}
          {t.ante > 0 && `, ante ${amount(t.ante, t.units)}`}
          {t.level !== null && ` · level ${t.level}`}
        </span>
        <span className={cn("font-mono tabular", t.hero_total_net >= 0 ? "text-accent-strong" : "text-danger")}>
          Session {t.hero_total_net >= 0 ? "+" : "−"}
          {amount(Math.abs(t.hero_total_net), t.units)}
        </span>
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto"
          disabled={busy || over}
          onClick={() => call("leave")}
          icon={<DoorOpen className="size-3.5" />}
        >
          Leave table
        </Button>
      </div>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
        {t.seats.map((s, i) => (
          <Seat key={s.name} seat={s} units={t.units} toAct={t.to_act === i} />
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-4 rounded-md border border-line bg-surface-2/40 px-3 py-3">
        <div className="flex min-h-8 items-center gap-1">
          {t.board.length ? <CardRow cards={t.board} /> : <span className="text-xs text-subtle">No board yet</span>}
        </div>
        {t.status === "awaiting_hero" && (
          <span className="font-mono text-sm tabular text-fg">Pot {amount(t.pot, t.units)}</span>
        )}
        {t.status === "hand_over" && t.last_hand_net !== null && (
          <span className={cn("text-sm", t.last_hand_net >= 0 ? "text-accent-strong" : "text-danger")}>
            You {t.last_hand_net >= 0 ? "won" : "lost"} {amount(Math.abs(t.last_hand_net), t.units)} this hand
          </span>
        )}
      </div>
      {t.status === "awaiting_hero" && t.legal && (
        <Controls key={t.log.length} t={t} onAct={(a) => call("act", { action: a })} busy={busy} />
      )}
      {t.status === "hand_over" && (
        <div>
          <Button variant="primary" loading={busy} onClick={() => call("next")} icon={<Play className="size-4" />}>
            {t.hand_no === 0 ? "Deal the first hand" : "Next hand"}
          </Button>
        </div>
      )}
      {over && <p className="text-sm text-fg">The session is over. The report is below.</p>}
      {error && <ErrorState message={error} />}
      {t.log.length > 0 && (
        <ol className="max-h-48 overflow-y-auto rounded-md border border-line px-3 py-2 text-xs" aria-label="Hand log">
          {t.log.map((l, i) => (
            <li key={i} className="flex gap-2">
              <span className="w-12 shrink-0 text-subtle capitalize">{l.street}</span>
              <span className="text-fg">{l.player}</span>
              <span className="text-muted">{l.text}</span>
            </li>
          ))}
        </ol>
      )}
      {t.notes.map((n) => (
        <p key={n} className="text-xs text-subtle">
          {n}
        </p>
      ))}
    </div>
  );
}

"use client";

import { type SeatView, type TableView, amount } from "@/lib/play";
import { PlayingCard } from "@/components/poker/CardText";
import { cn } from "@/components/ui/cn";

/**
 * Where seat `k` of `n` sits on the oval, as an angle in degrees (screen coordinates, y down):
 * you (seat 0) at the bottom, the others clockwise in the order the engine deals and acts.
 */
export function seatAngle(k: number, n: number): number {
  return 90 + (k * 360) / n;
}

/**
 * A point on the ellipse at `scale` of the seat radius. The radii are CSS variables on the table
 * box (narrower on phones so the side seats stay on screen).
 */
function at(angle: number, scale: number): { left: string; top: string } {
  const r = (angle * Math.PI) / 180;
  const x = (scale * Math.cos(r)).toFixed(4);
  const y = (scale * Math.sin(r)).toFixed(4);
  return { left: `calc(50% + var(--rx) * ${x})`, top: `calc(50% + var(--ry) * ${y})` };
}

function CardBack({ big }: { big?: boolean }) {
  return (
    <span
      aria-hidden
      className={cn(
        "inline-block rounded-[3px] border border-white/40 shadow-sm",
        big ? "h-8 w-6" : "h-6 w-[1.1rem]",
        "bg-[repeating-linear-gradient(45deg,#7a1f2b_0_3px,#9b2c3a_3px_6px)]",
      )}
    />
  );
}

function Cards({ seat, showAll, big }: { seat: SeatView; showAll: boolean; big: boolean }) {
  const cards = seat.cards ?? (showAll ? seat.hidden_cards : null);
  const size = big ? "md" : "sm";
  if (cards) {
    return (
      <span
        className={cn("inline-flex gap-0.5", !seat.cards && "opacity-70")}
        title={!seat.cards ? "Not shown at the table" : undefined}
      >
        {cards.map((c) => (
          <PlayingCard key={c} card={c} size={size} />
        ))}
      </span>
    );
  }
  if (seat.in_hand && !seat.folded) {
    return (
      <span className="inline-flex gap-0.5" aria-label="Face-down cards">
        <CardBack big={big} />
        <CardBack big={big} />
      </span>
    );
  }
  return <span className={big ? "h-8" : "h-6"} />;
}

function SeatPod({
  seat,
  units,
  toAct,
  showAll,
  hideStack,
}: {
  seat: SeatView;
  units: TableView["units"];
  toAct: boolean;
  showAll: boolean;
  hideStack?: boolean;
}) {
  const out = seat.folded || seat.busted;
  return (
    <div
      data-testid="seat"
      className={cn("flex w-[5.6rem] flex-col items-center gap-0.5 sm:w-32", out && "opacity-45")}
    >
      <Cards seat={seat} showAll={showAll} big={seat.is_hero} />
      <div
        className={cn(
          "w-full rounded-lg border px-1.5 py-1 text-center shadow-md backdrop-blur-sm",
          "border-white/15 bg-black/55 text-white",
          seat.is_hero && "border-accent/70",
          toAct && "ring-2 ring-accent ring-offset-1 ring-offset-transparent motion-safe:animate-pulse",
        )}
      >
        <p className={cn("truncate text-[11px] font-medium sm:text-xs", seat.is_hero && "text-accent-strong")}>
          {seat.name}
        </p>
        <p className="flex items-center justify-center gap-1 font-mono text-[11px] text-white/80 tabular sm:text-xs">
          {seat.position && seat.position !== seat.name && <span className="text-white/55">{seat.position}</span>}
          {!hideStack && (
            <span>{seat.busted ? `out${seat.finish ? ` (${seat.finish})` : ""}` : amount(seat.stack, units)}</span>
          )}
        </p>
      </div>
      <p
        className="min-h-4 max-w-full truncate text-center text-[11px] text-white/85"
        aria-label={`${seat.name} last action`}
      >
        {seat.all_in ? <span className="font-semibold text-red-300 uppercase">all-in</span> : seat.last_action}
      </p>
    </div>
  );
}

/** A stack of chips with the amount, in front of a seat. */
function Bet({ value, units }: { value: number; units: TableView["units"] }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-black/45 py-0.5 pr-2 pl-0.5 font-mono text-[11px] text-amber-100 tabular">
      <span aria-hidden className="size-3.5 rounded-full border-2 border-dashed border-white bg-amber-500" />
      {amount(value, units)}
    </span>
  );
}

/** The oval table: seats, bets, dealer button, board and pot. */
/** `hideStacks`: seats without a stack (e.g. an Academy question that does not give stacks). */
export function TableFelt({ t, showAll, hideStacks }: { t: TableView; showAll: boolean; hideStacks?: boolean }) {
  const n = t.seats.length;
  const slots = Array.from({ length: 5 }, (_, i) => t.board[i] ?? null);
  // `t.pot` counts this street's bets; the middle shows the pot without them (they sit in front).
  const bets = t.seats.reduce((sum, s) => sum + s.street_commit, 0);
  return (
    <div className="relative mx-auto w-full max-w-3xl px-1 pt-6 pb-10 sm:px-10 sm:pt-10 sm:pb-12">
      <div
        className="relative aspect-[4/5] w-full [--rx:35%] [--ry:42%] sm:aspect-[16/9] sm:[--rx:44%] sm:[--ry:41%]"
        aria-label="Table"
      >
        {/* Rail and felt. */}
        <div
          aria-hidden
          className="absolute inset-[7%] rounded-[50%] border-[10px] border-[#4a3222] shadow-[0_10px_30px_rgba(0,0,0,0.45),inset_0_0_40px_rgba(0,0,0,0.55)] sm:border-[14px]"
          style={{ background: "radial-gradient(ellipse at center, #237a52 0%, #17583a 60%, #0f3f2a 100%)" }}
        />
        {/* Board and pot. */}
        <div className="absolute top-1/2 left-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-1.5">
          <div className="flex gap-1" aria-label="Board">
            {slots.map((c, i) =>
              c ? (
                <PlayingCard key={i} card={c} />
              ) : (
                <span key={i} aria-hidden className="h-8 w-7 rounded-[4px] border border-dashed border-white/20" />
              ),
            )}
          </div>
          {t.status === "awaiting_hero" && t.pot > 0 && (
            <span className="flex flex-col items-center gap-0.5">
              <span className="rounded-full bg-black/45 px-2.5 py-0.5 font-mono text-xs text-white tabular">
                Pot {amount(t.pot - bets, t.units)}
              </span>
              {bets > 0 && (
                <span className="font-mono text-[10px] text-white/70 tabular">
                  total {amount(t.pot, t.units)} with bets
                </span>
              )}
            </span>
          )}
          {t.status === "hand_over" && t.last_hand_net !== null && (
            <span
              className={cn(
                "rounded-full bg-black/55 px-2.5 py-0.5 text-xs font-medium",
                t.last_hand_net >= 0 ? "text-emerald-300" : "text-red-300",
              )}
            >
              You {t.last_hand_net >= 0 ? "won" : "lost"} {amount(Math.abs(t.last_hand_net), t.units)}
            </span>
          )}
        </div>
        {t.seats.map((s, k) => {
          const a = seatAngle(k, n);
          const seatPos = at(a, 1);
          const betPos = at(a, 0.5);
          const buttonPos = at(a - 360 / n / 3.2, 0.7);
          return (
            <div key={s.name} className="contents">
              <div className="absolute z-10 -translate-x-1/2 -translate-y-1/2" style={seatPos}>
                <SeatPod
                  seat={s}
                  units={t.units}
                  toAct={t.to_act === k}
                  showAll={showAll}
                  hideStack={hideStacks && !(s.stack > 0)}
                />
              </div>
              {s.street_commit > 0 && (
                <div className="absolute -translate-x-1/2 -translate-y-1/2" style={betPos}>
                  <Bet value={s.street_commit} units={t.units} />
                </div>
              )}
              {s.is_button && (
                <span
                  aria-label="Dealer button"
                  className="absolute flex size-5 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-black/30 bg-white text-[10px] font-bold text-black shadow"
                  style={buttonPos}
                >
                  D
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

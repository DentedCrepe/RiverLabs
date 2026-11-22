import { SUIT_SYMBOL, type SuitCode, cardName, splitCards } from "@/lib/poker/cards";
import { cn } from "@/components/ui/cn";

const SUIT_CLASS: Record<SuitCode, string> = {
  s: "text-suit-s",
  h: "text-suit-h",
  d: "text-suit-d",
  c: "text-suit-c",
};

export function PlayingCard({ card, size = "md" }: { card: string; size?: "sm" | "md" }) {
  const rank = card[0]?.toUpperCase() ?? "?";
  const suit = (card[1]?.toLowerCase() ?? "s") as SuitCode;
  return (
    <span
      role="img"
      aria-label={cardName(card)}
      className={cn(
        "inline-flex items-center justify-center rounded border border-line-strong bg-surface-3 font-mono font-semibold",
        size === "sm" ? "h-6 w-[1.35rem] text-[11px]" : "h-8 w-7 text-sm",
        SUIT_CLASS[suit] ?? "text-fg",
      )}
    >
      {rank === "T" ? "T" : rank}
      <span aria-hidden className="text-[0.8em]">
        {SUIT_SYMBOL[suit] ?? "?"}
      </span>
    </span>
  );
}

export function CardRow({ cards, size }: { cards: string | string[]; size?: "sm" | "md" }) {
  const list = Array.isArray(cards) ? cards : splitCards(cards);
  return (
    <span className="inline-flex gap-1">
      {list.map((c, i) => (
        <PlayingCard key={`${c}-${i}`} card={c} size={size} />
      ))}
    </span>
  );
}

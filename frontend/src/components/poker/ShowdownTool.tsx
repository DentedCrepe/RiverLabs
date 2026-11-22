"use client";

import { Plus, Trophy, X } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ApiError, api } from "@/lib/api";
import type { CompareResult } from "@/lib/types";
import { Button } from "@/components/ui/Button";
import { Field, TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";
import { CardRow } from "./CardText";

/** Showdown evaluation on a complete board, computed by the engine's 7-card evaluator. */
export function ShowdownTool({ compact = false }: { compact?: boolean }) {
  const [board, setBoard] = useState("Ks Td 7h 4c 2s");
  const [hands, setHands] = useState(["Ah Kd", "Tc Th"]);
  const [result, setResult] = useState<CompareResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const strip = (s: string) => s.replace(/\s+/g, "");
      setResult(
        await api.post<CompareResult>("/api/v1/hands/compare", { board: strip(board), hands: hands.map(strip) }),
      );
    } catch (err) {
      setResult(null);
      setError(err instanceof ApiError ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-4" aria-label="Showdown evaluator">
      <div className={compact ? "grid gap-3" : "grid gap-3 sm:grid-cols-[2fr_3fr]"}>
        <Field label="Board (5 cards)">
          {(id, d) => (
            <TextInput id={id} aria-describedby={d} value={board} onChange={(e) => setBoard(e.target.value)} />
          )}
        </Field>
        <div className="grid grid-cols-2 gap-3">
          {hands.map((h, i) => (
            <Field key={i} label={`Player ${i + 1}`}>
              {(id) => (
                <div className="flex gap-1">
                  <TextInput
                    id={id}
                    value={h}
                    onChange={(e) => setHands(hands.map((x, j) => (j === i ? e.target.value : x)))}
                  />
                  {hands.length > 2 && (
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label={`Remove player ${i + 1}`}
                      className="h-8"
                      onClick={() => setHands(hands.filter((_, j) => j !== i))}
                      icon={<X className="size-3.5" />}
                    />
                  )}
                </div>
              )}
            </Field>
          ))}
        </div>
      </div>
      <div className="flex gap-2">
        <Button type="submit" variant="primary" loading={loading}>
          Evaluate
        </Button>
        {hands.length < 6 && (
          <Button onClick={() => setHands([...hands, ""])} icon={<Plus className="size-4" />}>
            Player
          </Button>
        )}
      </div>
      {error && <ErrorState title="The engine rejected this input" message={error} />}
      {result && (
        <ul className="divide-y divide-line rounded-md border border-line" aria-live="polite">
          {result.results.map((r, i) => {
            const won = result.winners.includes(i);
            return (
              <li key={i} className="flex flex-wrap items-center gap-3 px-3 py-2 text-sm">
                <span className="w-16 text-muted">Player {i + 1}</span>
                <CardRow cards={r.best_five} size="sm" />
                <span className={won ? "font-medium text-fg" : "text-muted"}>{r.description}</span>
                {won && (
                  <span className="ml-auto inline-flex items-center gap-1 text-xs text-accent-strong">
                    <Trophy aria-hidden className="size-3.5" />
                    {result.winners.length > 1 ? "Split" : "Wins"}
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </form>
  );
}

"use client";

import { Plus, X } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ApiError, api } from "@/lib/api";
import type { EquityResult } from "@/lib/types";
import { formatPercent } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { Field, Select, TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";

/** All-in equity for 2–6 ranges or hands, computed by the engine. */
export function EquityTool() {
  const [ranges, setRanges] = useState(["AsKs", "QQ+,AQs+"]);
  const [board, setBoard] = useState("Qh 7s 2s");
  const [dead, setDead] = useState("");
  const [method, setMethod] = useState<"exact" | "monte_carlo">("exact");
  const [iterations, setIterations] = useState("200000");
  const [seed, setSeed] = useState("1");
  const [result, setResult] = useState<EquityResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    const strip = (s: string) => s.replace(/\s+/g, "");
    try {
      setResult(
        await api.post<EquityResult>("/api/v1/equity", {
          ranges: ranges.map((r) => r.trim()),
          board: strip(board),
          dead: strip(dead),
          method,
          iterations: Number(iterations),
          seed: Number(seed),
        }),
      );
    } catch (err) {
      setResult(null);
      setError(err instanceof ApiError ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-4" aria-label="Equity calculator">
      <div className="grid gap-3 sm:grid-cols-2">
        {ranges.map((r, i) => (
          <Field
            key={i}
            label={`Player ${i + 1} range or hand`}
            hint={i === 0 ? "e.g. AsKs, QQ+, ATs+, 76s:0.5" : undefined}
          >
            {(id, d) => (
              <div className="flex gap-1">
                <TextInput
                  id={id}
                  aria-describedby={d}
                  value={r}
                  onChange={(e) => setRanges(ranges.map((x, j) => (j === i ? e.target.value : x)))}
                />
                {ranges.length > 2 && (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-8"
                    aria-label={`Remove player ${i + 1}`}
                    onClick={() => setRanges(ranges.filter((_, j) => j !== i))}
                    icon={<X className="size-3.5" />}
                  />
                )}
              </div>
            )}
          </Field>
        ))}
      </div>
      <div className="grid gap-3 sm:grid-cols-5">
        <Field label="Board">
          {(id) => <TextInput id={id} value={board} onChange={(e) => setBoard(e.target.value)} />}
        </Field>
        <Field label="Dead cards">
          {(id) => <TextInput id={id} value={dead} onChange={(e) => setDead(e.target.value)} />}
        </Field>
        <Field label="Method">
          {(id) => (
            <Select id={id} value={method} onChange={(e) => setMethod(e.target.value as typeof method)}>
              <option value="exact">Exact enumeration</option>
              <option value="monte_carlo">Monte Carlo (seeded)</option>
            </Select>
          )}
        </Field>
        <Field label="Iterations">
          {(id) => (
            <TextInput
              id={id}
              inputMode="numeric"
              disabled={method === "exact"}
              value={iterations}
              onChange={(e) => setIterations(e.target.value)}
            />
          )}
        </Field>
        <Field label="Seed">
          {(id) => (
            <TextInput
              id={id}
              inputMode="numeric"
              disabled={method === "exact"}
              value={seed}
              onChange={(e) => setSeed(e.target.value)}
            />
          )}
        </Field>
      </div>
      <div className="flex gap-2">
        <Button type="submit" variant="primary" loading={loading}>
          Calculate equity
        </Button>
        {ranges.length < 6 && (
          <Button onClick={() => setRanges([...ranges, ""])} icon={<Plus className="size-4" />}>
            Player
          </Button>
        )}
      </div>
      {error && <ErrorState title="Equity could not be calculated" message={error} />}
      {result && (
        <div aria-live="polite" className="overflow-x-auto">
          <table className="w-full min-w-[480px] text-sm">
            <caption className="mb-2 text-left text-xs text-muted">
              {result.method === "exact"
                ? `Exact: ${result.samples.toLocaleString()} weighted deals enumerated.`
                : `Monte Carlo: ${result.samples.toLocaleString()} samples, seed ${result.seed}. ± is one standard error.`}
            </caption>
            <thead>
              <tr className="border-b border-line text-left">
                <th className="label-caps py-1.5 font-medium">Player</th>
                <th className="label-caps py-1.5 font-medium">Equity</th>
                <th className="label-caps py-1.5 font-medium">Win</th>
                <th className="label-caps py-1.5 font-medium">Tie</th>
                <th className="label-caps w-1/3 py-1.5 font-medium" aria-hidden />
              </tr>
            </thead>
            <tbody className="font-mono tabular">
              {result.equity.map((eq, i) => (
                <tr key={i} className="border-b border-line/60">
                  <td className="py-1.5 font-sans text-muted">
                    {i + 1}. <span className="text-fg">{ranges[i]}</span>
                  </td>
                  <td className="py-1.5 text-fg">
                    {formatPercent(eq)}
                    {result.std_error && <span className="text-subtle"> ±{formatPercent(result.std_error[i])}</span>}
                  </td>
                  <td className="py-1.5 text-muted">{formatPercent(result.win[i])}</td>
                  <td className="py-1.5 text-muted">{formatPercent(result.tie[i])}</td>
                  <td className="py-1.5">
                    <div className="h-1.5 rounded bg-surface-3">
                      <div className="h-1.5 rounded bg-accent" style={{ width: `${eq * 100}%` }} />
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </form>
  );
}

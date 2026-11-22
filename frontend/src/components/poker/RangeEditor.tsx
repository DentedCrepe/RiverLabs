"use client";

import { Eraser, Upload } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ApiError, api } from "@/lib/api";
import type { RangeAnalysis } from "@/lib/types";
import { formatPercent } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { RangeMatrix } from "./RangeMatrix";

const BRUSHES = [1, 0.75, 0.5, 0.25] as const;

/** Builds engine range notation from per-class weights ("AKs", "AKs:0.5", ...). */
export function weightsToNotation(weights: Readonly<Record<string, number>>): string {
  return Object.entries(weights)
    .filter(([, w]) => w > 0)
    .map(([label, w]) => (w === 1 ? label : `${label}:${w}`))
    .join(",");
}

/**
 * User-defined range editing. The weights are whatever the user enters — they are not
 * strategies. Combo counts, card removal and canonical notation come from the engine.
 */
export function RangeEditor() {
  const [weights, setWeights] = useState<Record<string, number>>({});
  const [brush, setBrush] = useState<number>(1);
  const [dead, setDead] = useState("");
  const [importText, setImportText] = useState("");
  const [analysis, setAnalysis] = useState<RangeAnalysis | null>(null);
  const [error, setError] = useState<string | null>(null);
  const notation = useMemo(() => weightsToNotation(weights), [weights]);

  useEffect(() => {
    const handle = setTimeout(() => {
      api
        .post<RangeAnalysis>("/api/v1/ranges/analyze", { notation, dead: dead.replace(/\s+/g, "") })
        .then((a) => {
          setAnalysis(a);
          setError(null);
        })
        .catch((e: unknown) => setError(e instanceof ApiError ? e.message : String(e)));
    }, 250);
    return () => clearTimeout(handle);
  }, [notation, dead]);

  async function importRange() {
    try {
      const a = await api.post<RangeAnalysis>("/api/v1/ranges/analyze", { notation: importText, dead: "" });
      const next: Record<string, number> = {};
      for (const c of a.matrix) if (c.weight_sum > 0) next[c.label] = Number((c.weight_sum / c.combos).toFixed(4));
      setWeights(next);
      setError(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    }
  }

  const liveByLabel = useMemo(() => new Map(analysis?.matrix.map((c) => [c.label, c]) ?? []), [analysis]);

  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_340px]">
      <Card>
        <CardHeader
          title="Range matrix"
          subtitle="Click or drag to paint. Weights are your own inputs, not solver frequencies."
          actions={
            <Button size="sm" variant="ghost" onClick={() => setWeights({})} icon={<Eraser className="size-3.5" />}>
              Clear
            </Button>
          }
        />
        <CardBody className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center gap-2" role="radiogroup" aria-label="Brush weight">
            <span className="label-caps mr-1">Brush</span>
            {BRUSHES.map((b) => (
              <button
                key={b}
                type="button"
                role="radio"
                aria-checked={brush === b}
                onClick={() => setBrush(b)}
                className={cn(
                  "h-7 rounded-md border px-2.5 font-mono text-xs",
                  brush === b
                    ? "border-accent bg-accent-dim text-accent-strong"
                    : "border-line text-muted hover:text-fg",
                )}
              >
                {b * 100}%
              </button>
            ))}
          </div>
          <RangeMatrix
            weights={weights}
            brush={brush}
            onPaint={(label, w) => setWeights((prev) => ({ ...prev, [label]: w }))}
            annotate={(cell) => {
              const c = liveByLabel.get(cell.label);
              return c && c.live_combos !== c.combos ? `${c.live_combos}/${c.combos}` : undefined;
            }}
          />
        </CardBody>
      </Card>

      <div className="flex flex-col gap-6">
        <Card>
          <CardHeader title="Range statistics" subtitle="Computed by the engine" />
          <CardBody>
            <dl className="grid grid-cols-2 gap-x-4 gap-y-3 font-mono text-sm tabular">
              <dt className="label-caps self-center font-sans">Combos</dt>
              <dd className="text-right text-fg">{analysis ? analysis.combos.toFixed(2) : "—"}</dd>
              <dt className="label-caps self-center font-sans">Of all hands</dt>
              <dd className="text-right text-fg">{analysis ? formatPercent(analysis.fraction_of_all_hands) : "—"}</dd>
              <dt className="label-caps self-center font-sans">Live combos</dt>
              <dd className="text-right text-fg">{analysis ? analysis.live_combos.toFixed(2) : "—"}</dd>
              <dt className="label-caps self-center font-sans">Blocked</dt>
              <dd className="text-right text-fg">{analysis ? formatPercent(analysis.blocked_fraction) : "—"}</dd>
            </dl>
            <div className="mt-4">
              <Field label="Dead / board cards" hint="Card removal and blocker effects">
                {(id, d) => (
                  <TextInput
                    id={id}
                    aria-describedby={d}
                    placeholder="e.g. As Kd 7h"
                    value={dead}
                    onChange={(e) => setDead(e.target.value)}
                  />
                )}
              </Field>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Notation" />
          <CardBody className="flex flex-col gap-3">
            <p className="min-h-10 rounded-md border border-line bg-canvas p-2 font-mono text-xs break-words text-fg">
              {analysis?.notation || <span className="text-subtle">Empty range</span>}
            </p>
            <Field label="Import notation" hint="Combo-specific weights (e.g. AsKs) are shown as class averages.">
              {(id, d) => (
                <TextInput
                  id={id}
                  aria-describedby={d}
                  placeholder="TT+,AQs+,KQs,AKo"
                  value={importText}
                  onChange={(e) => setImportText(e.target.value)}
                />
              )}
            </Field>
            <Button onClick={importRange} icon={<Upload className="size-4" />} disabled={!importText.trim()}>
              Load into matrix
            </Button>
            {error && <ErrorState title="Range error" message={error} />}
          </CardBody>
        </Card>
      </div>
    </div>
  );
}

"use client";

import { CircleAlert } from "lucide-react";
import { type PlayReport as Report, type Ratio, pct } from "@/lib/play";
import { useApi } from "@/lib/useApi";
import { ErrorState, EmptyState, LoadingState } from "@/components/ui/States";
import { Metric } from "@/components/solver/ResultView";
import { cn } from "@/components/ui/cn";

const STATS: [keyof Report["stats"], string, string][] = [
  ["vpip", "VPIP", "Put money in voluntarily preflop"],
  ["pfr", "PFR", "Raised preflop"],
  ["three_bet", "3-bet", "Re-raised when facing one raise"],
  ["fold_to_three_bet", "Fold to 3-bet", "After opening"],
  ["cbet", "C-bet", "Bet the flop as the preflop raiser"],
  ["fold_to_cbet", "Fold to c-bet", "Facing a bot's continuation bet"],
  ["went_to_showdown", "Went to showdown", "Of hands where you saw the flop"],
  ["won_at_showdown", "Won at showdown", "Of your showdowns"],
];

const signed = (x: number, d = 1) => `${x >= 0 ? "+" : "−"}${Math.abs(x).toFixed(d)}`;

export function ReportView({ report: r }: { report: Report }) {
  if (r.hands === 0) return <EmptyState title="No hands yet" description="Play at least one hand to get a report." />;
  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
        <Metric label="Hands" value={r.hands.toLocaleString()} />
        <Metric
          label="Result"
          value={`${signed(r.net_bb)}bb`}
          sub={r.units === "chips" ? `${signed(r.net, 0)} chips` : undefined}
        />
        <Metric label="bb / 100 hands" value={r.bb_per_100 === null ? "—" : signed(r.bb_per_100)} />
        <Metric
          label="All-in luck"
          value={r.all_in ? `${signed(r.all_in.actual_bb - r.all_in.expected_bb)}bb` : "—"}
          sub={
            r.all_in
              ? `${r.all_in.hands} all-ins: won ${signed(r.all_in.actual_bb)}bb, expected ${signed(r.all_in.expected_bb)}bb`
              : "No all-ins before the river"
          }
        />
      </div>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold text-fg">Your statistics</h3>
        <dl className="grid grid-cols-2 gap-x-6 gap-y-1.5 text-sm sm:grid-cols-4">
          {STATS.map(([key, label, hint]) => {
            const v = r.stats[key] as Ratio;
            return (
              <div key={key} title={hint}>
                <dt className="text-xs text-muted">{label}</dt>
                <dd className="font-mono text-fg tabular">
                  {pct(v)}{" "}
                  <span className="text-xs text-subtle">
                    ({v.made}/{v.opportunities})
                  </span>
                </dd>
              </div>
            );
          })}
          <div>
            <dt className="text-xs text-muted">Aggression factor</dt>
            <dd className="font-mono text-fg tabular">{r.stats.aggression_factor?.toFixed(2) ?? "—"}</dd>
          </div>
        </dl>
        {r.model_comparison && (
          <p className="text-xs text-muted">
            In {r.model_comparison.decisions} preflop decisions the model could judge, you played (called or raised){" "}
            {(r.model_comparison.voluntary_you * 100).toFixed(0)}% where the model would have played{" "}
            {(r.model_comparison.voluntary_model * 100).toFixed(0)}% of the same hands, and raised{" "}
            {(r.model_comparison.raise_you * 100).toFixed(0)}% against its{" "}
            {(r.model_comparison.raise_model * 100).toFixed(0)}%.
          </p>
        )}
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold text-fg">Weaknesses found</h3>
        {r.findings.length === 0 ? (
          <p className="text-sm text-muted">No call, fold or preflop choice stood out against the references used.</p>
        ) : (
          <ul className="flex flex-col gap-3">
            {r.findings.map((f) => (
              <li key={f.key} className="rounded-md border border-line px-3 py-2">
                <div className="flex flex-wrap items-baseline justify-between gap-2">
                  <h4 className="text-sm font-medium text-fg">{f.title}</h4>
                  <span className="font-mono text-xs text-warn tabular">
                    {f.count} of {f.out_of} decisions
                  </span>
                </div>
                <p className="text-xs text-muted">{f.detail}</p>
                <p className="text-xs text-subtle">Measured against: {f.measured_against}</p>
                {f.examples.length > 0 && (
                  <ul className="mt-1 flex flex-col gap-0.5 font-mono text-[11px] text-muted">
                    {f.examples.map((e, i) => (
                      <li key={i}>
                        Hand {e.hand_no}: {e.text}
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold text-fg">By position</h3>
        <table className="w-full max-w-md text-sm">
          <thead className="text-left text-xs text-muted">
            <tr>
              <th className="font-normal">Position</th>
              <th className="font-normal">Hands</th>
              <th className="font-normal">VPIP</th>
              <th className="font-normal">Result</th>
            </tr>
          </thead>
          <tbody className="font-mono tabular">
            {r.by_position.map((p) => (
              <tr key={p.position}>
                <td>{p.position}</td>
                <td>{p.hands}</td>
                <td>{pct(p.vpip)}</td>
                <td className={cn(p.net_bb >= 0 ? "text-accent-strong" : "text-danger")}>{signed(p.net_bb)}bb</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <ul className="flex flex-col gap-1">
        {r.caveats.map((c) => (
          <li key={c} className="flex items-start gap-2 text-xs text-muted">
            <CircleAlert aria-hidden className="mt-px size-3.5 shrink-0 text-warn" />
            {c}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function PlayReport({ sessionId, refreshKey }: { sessionId: string; refreshKey: number }) {
  const { data, error, loading } = useApi<Report>(`/api/v1/play/sessions/${sessionId}/report?v=${refreshKey}`);
  if (error) return <ErrorState message={error.message} />;
  if (loading && !data) return <LoadingState label="Building the report…" />;
  return data ? <ReportView report={data} /> : null;
}

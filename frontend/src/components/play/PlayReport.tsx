"use client";

import { CircleAlert } from "lucide-react";
import { useState } from "react";
import { LEAKS, type Leak, type LeakPractice, type PlayReport as Report, type Ratio, pct } from "@/lib/play";
import { GLOSSARY } from "@/lib/glossary";
import { useApi } from "@/lib/useApi";
import { InfoTip } from "@/components/ui/InfoTip";
import { ErrorState, EmptyState, LoadingState } from "@/components/ui/States";
import { Metric } from "@/components/solver/ResultView";
import { cn } from "@/components/ui/cn";
import { Button } from "@/components/ui/Button";
import { FindingBrowser } from "./FindingBrowser";

const STATS: [keyof Report["stats"], string, string][] = [
  ["vpip", "VPIP", GLOSSARY.vpip],
  ["pfr", "PFR", GLOSSARY.pfr],
  ["three_bet", "3-bet", GLOSSARY.threeBet],
  ["fold_to_three_bet", "Fold to 3-bet", GLOSSARY.foldToThreeBet],
  ["cbet", "C-bet", GLOSSARY.cbet],
  ["fold_to_cbet", "Fold to c-bet", GLOSSARY.foldToCbet],
  ["went_to_showdown", "Went to showdown", GLOSSARY.wtsd],
  ["won_at_showdown", "Won at showdown", GLOSSARY.wsd],
];

const signed = (x: number, d = 1) => `${x >= 0 ? "+" : "−"}${Math.abs(x).toFixed(d)}`;

const LEAK_LABEL = Object.fromEntries(LEAKS.map((l) => [l.value, l.label])) as Record<Leak, string>;

/** Early against late: did matching the best response improve as the session went on? */
function Adjustment({ timeline }: { timeline: [number, boolean][] }) {
  if (timeline.length < 4) return null;
  const half = Math.floor(timeline.length / 2);
  const rate = (xs: [number, boolean][]) => `${xs.filter(([, ok]) => ok).length} of ${xs.length}`;
  return (
    <p className="text-xs text-muted">
      Adjusting: first half of those decisions {rate(timeline.slice(0, half))} matched, second half{" "}
      {rate(timeline.slice(half))}.
    </p>
  );
}

/** Spot the leak: guess each bot's leak before the answers (and the leak sections) appear. */
function SpotTheLeak({
  bots,
  revealed,
  onReveal,
}: {
  bots: { name: string; leak: Leak }[];
  revealed: Record<string, Leak> | null;
  onReveal: (guesses: Record<string, Leak>) => void;
}) {
  const [guess, setGuess] = useState<Record<string, Leak>>(() =>
    Object.fromEntries(bots.map((b) => [b.name, "none" as Leak])),
  );
  const right = revealed ? bots.filter((b) => revealed[b.name] === b.leak).length : 0;
  return (
    <section className="flex flex-col gap-2 rounded-md border border-line px-3 py-2" aria-label="Spot the leak">
      <h3 className="text-sm font-semibold text-fg">Spot the leak</h3>
      <p className="text-xs text-muted">
        Each bot had one river leak, or none. Guess them, then reveal the answers and how your river play against each
        one compared with the best response.
      </p>
      <ul className="flex flex-col gap-1.5">
        {bots.map((b) => (
          <li key={b.name} className="flex flex-wrap items-center gap-2 text-sm">
            <span className="min-w-28 text-fg">{b.name}</span>
            {revealed ? (
              <span className={cn(revealed[b.name] === b.leak ? "text-accent-strong" : "text-danger")}>
                {revealed[b.name] === b.leak ? "Right" : "Not quite"}: {LEAK_LABEL[b.leak]}
                {revealed[b.name] !== b.leak && ` (you said ${LEAK_LABEL[revealed[b.name]].toLowerCase()})`}
              </span>
            ) : (
              <select
                aria-label={`Leak of ${b.name}`}
                value={guess[b.name]}
                onChange={(e) => setGuess({ ...guess, [b.name]: e.target.value as Leak })}
                className="h-8 rounded-md border border-line bg-canvas px-2 text-sm text-fg"
              >
                {LEAKS.map((l) => (
                  <option key={l.value} value={l.value}>
                    {l.label}
                  </option>
                ))}
              </select>
            )}
          </li>
        ))}
      </ul>
      {revealed ? (
        <p className="font-mono text-sm text-fg tabular">
          {right} of {bots.length} right
        </p>
      ) : (
        <div>
          <Button size="sm" onClick={() => onReveal(guess)}>
            Reveal the leaks
          </Button>
        </div>
      )}
    </section>
  );
}

function leakKey(sessionId?: string) {
  return sessionId ? `riverlab.leakGuess.${sessionId}` : null;
}

export function ReportView({ report: r, sessionId }: { report: Report; sessionId?: string }) {
  const [browsing, setBrowsing] = useState<string | null>(null);
  const hidden = r.hidden_leaks ?? [];
  const [revealed, setRevealed] = useState<Record<string, Leak> | null>(() => {
    const key = leakKey(sessionId);
    try {
      const saved = key ? window.localStorage.getItem(key) : null;
      return saved ? (JSON.parse(saved) as Record<string, Leak>) : null;
    } catch {
      return null;
    }
  });
  const reveal = (g: Record<string, Leak>) => {
    setRevealed(g);
    const key = leakKey(sessionId);
    try {
      if (key) window.localStorage.setItem(key, JSON.stringify(g));
    } catch {
      // not remembered: the answers stay shown until the page reloads
    }
  };
  const leakSections: LeakPractice[] = hidden.length > 0 && !revealed ? [] : (r.leak_practice ?? []);
  if (r.hands === 0) return <EmptyState title="No hands yet" description="Play at least one hand to get a report." />;
  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
        <Metric label="Hands" value={r.hands.toLocaleString()} info={GLOSSARY.hands} />
        <Metric
          label="Result"
          info={GLOSSARY.result}
          value={`${signed(r.net_bb)}bb`}
          sub={r.units === "chips" ? `${signed(r.net, 0)} chips` : undefined}
        />
        <Metric
          label="bb / 100 hands"
          value={r.bb_per_100 === null ? "—" : signed(r.bb_per_100)}
          info={GLOSSARY.bb100}
        />
        <Metric
          label="All-in luck"
          info={GLOSSARY.allInLuck}
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
              <div key={key}>
                <dt className="flex items-center gap-1 text-xs text-muted">
                  {label}
                  <InfoTip label={label}>{hint}</InfoTip>
                </dt>
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
            <dt className="flex items-center gap-1 text-xs text-muted">
              Aggression factor
              <InfoTip label="aggression factor">{GLOSSARY.aggression}</InfoTip>
            </dt>
            <dd className="font-mono text-fg tabular">{r.stats.aggression_factor?.toFixed(2) ?? "—"}</dd>
          </div>
        </dl>
        {r.model_comparison && (
          <p className="text-xs text-muted">
            <InfoTip label="the model comparison" className="mr-1">
              {GLOSSARY.modelComparison}
            </InfoTip>
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
                    {f.examples.slice(0, 3).map((e, i) => (
                      <li key={i}>
                        Hand {e.hand_no}: {e.text}
                      </li>
                    ))}
                  </ul>
                )}
                {f.examples.length > 0 && (
                  <div className="mt-2">
                    <Button
                      size="sm"
                      variant={browsing === f.key ? "ghost" : "secondary"}
                      onClick={() => setBrowsing(browsing === f.key ? null : f.key)}
                    >
                      {browsing === f.key ? "Close" : `Flick through these hands (${f.examples.length})`}
                    </Button>
                    {browsing === f.key && (
                      <div className="mt-2">
                        <FindingBrowser examples={f.examples} sessionId={sessionId} />
                      </div>
                    )}
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {hidden.length > 0 && <SpotTheLeak bots={hidden} revealed={revealed} onReveal={reveal} />}
      {leakSections.map((l) => (
        <section
          key={l.leak}
          className="flex flex-col gap-1 rounded-md border border-line px-3 py-2"
          aria-label={`Against: ${l.label}`}
        >
          <h3 className="text-sm font-semibold text-fg">Practice leak: bots that {l.label}</h3>
          <p className="text-xs text-muted">{l.best_response}</p>
          <p className="text-sm text-fg">
            River calls and folds facing its bet:{" "}
            <span className="font-mono tabular">
              {l.decisions === 0 ? "none yet" : `${l.matched} of ${l.decisions} matched the best response`}
            </span>
          </p>
          <p className="text-xs text-subtle">
            Best response for that decision: call exactly when your equity against the bot&apos;s actual range (Monte
            Carlo estimate) is at least the price; your call ends the betting.
          </p>
          <Adjustment timeline={l.timeline ?? []} />
          {l.examples.slice(0, 3).map((e, i) => (
            <p key={i} className="font-mono text-[11px] text-muted">
              Hand {e.hand_no}: {e.text}
            </p>
          ))}
          <p className="text-sm text-fg">
            Your river bets it faced:{" "}
            <span className="font-mono tabular">
              {l.bets === 0 || l.mean_fold_share === null || l.mean_break_even === null
                ? "none yet"
                : `${l.bets}; its range folded ${(l.mean_fold_share * 100).toFixed(0)}% on average, a pure bluff needed ${(l.mean_break_even * 100).toFixed(0)}% (${l.bets_profitable_as_bluffs} of ${l.bets} would have profited as pure bluffs)`}
            </span>
          </p>
        </section>
      ))}

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
  return data ? <ReportView report={data} sessionId={sessionId} /> : null;
}

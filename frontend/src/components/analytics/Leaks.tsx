"use client";

import { Cpu } from "lucide-react";
import { useEffect, useState } from "react";
import { ApiError, api } from "@/lib/api";
import { useApi } from "@/lib/useApi";
import { FindingBrowser } from "@/components/play/FindingBrowser";
import { Metric } from "@/components/solver/ResultView";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, Select } from "@/components/ui/Field";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";

type Bucket = { decisions: number; leaks: number; ev_lost_bb: number };
export type LeakSummary = {
  reviews: number;
  threshold_pct_pot: number;
  total: Bucket;
  by_street: (Bucket & { street: string })[];
  by_position: (Bucket & { position: string })[];
  by_category: { category: string; leaks: number; ev_lost_bb: number }[];
  costliest: {
    session_id: string;
    hand_no: number;
    street: string;
    decision: number;
    position: string;
    taken: string;
    best: string;
    ev_loss_bb: number;
    ev_loss_pct_pot: number;
    category: string;
  }[];
};
type Status = { reviews: Record<"queued" | "running" | "done" | "failed" | "interrupted", number> };

const perDecision = (b: Bucket) => (b.decisions ? `${(b.ev_lost_bb / b.decisions).toFixed(3)}bb` : "—");

function BucketRows({ rows }: { rows: (Bucket & { label: string })[] }) {
  return (
    <>
      {rows
        .filter((r) => r.decisions > 0)
        .map((r) => (
          <tr key={r.label} className="font-mono tabular">
            <td className="py-0.5 pr-3 font-sans text-fg capitalize">{r.label}</td>
            <td className="pr-3">{r.decisions}</td>
            <td className="pr-3">{r.leaks}</td>
            <td className="pr-3">{r.ev_lost_bb.toFixed(2)}bb</td>
            <td>{perDecision(r)}</td>
          </tr>
        ))}
    </>
  );
}

/** Solver reviews of your hands in bulk, and where the EV went. */
export function LeaksSection() {
  const status = useApi<Status>("/api/v1/play/reviews/status");
  const [tick, setTick] = useState(0);
  const leaks = useApi<LeakSummary>(`/api/v1/analytics/leaks?v=${status.data?.reviews.done ?? 0}-${tick}`);
  const [scope, setScope] = useState("flagged");
  const [limit, setLimit] = useState("10");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queuedNow, setQueuedNow] = useState<number | null>(null);
  const [browsing, setBrowsing] = useState(false);
  const pending = (status.data?.reviews.queued ?? 0) + (status.data?.reviews.running ?? 0);

  // While reviews are queued or running, refresh the status (and so the summary) every few seconds.
  const reloadStatus = status.reload;
  useEffect(() => {
    if (pending === 0) return;
    const t = setInterval(reloadStatus, 4000);
    return () => clearInterval(t);
  }, [pending, reloadStatus]);

  async function queue() {
    setBusy(true);
    setError(null);
    try {
      const r = await api.post<{ queued: unknown[] }>("/api/v1/play/reviews/batch", { scope, limit: Number(limit) });
      setQueuedNow(r.queued.length);
      status.reload();
      setTick((t) => t + 1);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  const l = leaks.data;
  return (
    <Card>
      <CardHeader
        title="Solver leaks"
        subtitle="Your hands reviewed by the solver: where the EV went, decision by decision"
        icon={<Cpu className="size-4" />}
      />
      <CardBody className="flex flex-col gap-4">
        <div className="flex flex-wrap items-end gap-2">
          <Field label="Hands to review">
            {(id) => (
              <Select id={id} value={scope} onChange={(e) => setScope(e.target.value)}>
                <option value="flagged">With a flagged or unusual decision</option>
                <option value="recent">Most recent</option>
                <option value="biggest">Biggest wins and losses</option>
              </Select>
            )}
          </Field>
          <Field label="How many">
            {(id) => (
              <Select id={id} value={limit} onChange={(e) => setLimit(e.target.value)}>
                {["5", "10", "20", "30"].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Button variant="primary" onClick={queue} loading={busy} icon={<Cpu className="size-4" />}>
            Review with the solver
          </Button>
        </div>
        <p className="text-xs text-muted" aria-live="polite">
          {pending > 0
            ? `${pending} hand review${pending === 1 ? "" : "s"} waiting or running, one at a time (a few minutes per hand). You can keep using RiverLab.`
            : queuedNow === 0
              ? "No new hands to review: only heads-up hands with decisions after the flop can be solved, and reviewed hands are skipped."
              : "Only heads-up decisions after the flop can be solved; hands already reviewed are skipped."}
        </p>
        {error && <ErrorState message={error} />}
        {leaks.error ? (
          <ErrorState message={leaks.error.message} />
        ) : !l ? (
          <LoadingState label="Loading…" />
        ) : l.total.decisions === 0 ? (
          <EmptyState
            title="No reviewed hands yet"
            description="Queue some hands above. When their reviews finish, your leaks appear here."
          />
        ) : (
          <>
            <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
              <Metric label="Hands reviewed" value={String(l.reviews)} />
              <Metric label="Decisions" value={String(l.total.decisions)} />
              <Metric
                label="Leaks"
                value={String(l.total.leaks)}
                sub={`losing more than ${l.threshold_pct_pot}% of the pot`}
              />
              <Metric label="EV lost / decision" value={perDecision(l.total)} sub="against the solutions" />
            </div>
            {l.by_category.length > 0 && (
              <div className="flex flex-col gap-1">
                <h4 className="text-xs font-semibold text-fg">By kind of mistake</h4>
                <ul className="flex flex-col gap-0.5 font-mono text-xs text-muted tabular" aria-label="Leaks by kind">
                  {l.by_category.map((c) => (
                    <li key={c.category} className="flex flex-wrap gap-3">
                      <span className="font-sans text-fg">{c.category}</span>
                      <span>
                        {c.leaks}× · {c.ev_lost_bb.toFixed(2)}bb
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
            <div className="overflow-x-auto">
              <table className="text-xs text-muted" aria-label="Solver leaks by street and position">
                <thead>
                  <tr className="text-left">
                    <th className="pr-3 font-normal" />
                    <th className="pr-3 font-normal">Decisions</th>
                    <th className="pr-3 font-normal">Leaks</th>
                    <th className="pr-3 font-normal">EV lost</th>
                    <th className="font-normal">Per decision</th>
                  </tr>
                </thead>
                <tbody>
                  <BucketRows rows={l.by_street.map((s) => ({ ...s, label: s.street }))} />
                  <BucketRows rows={l.by_position.map((p) => ({ ...p, label: p.position }))} />
                </tbody>
              </table>
            </div>
            {l.costliest.length > 0 && (
              <div className="flex flex-col gap-2">
                <div className="flex flex-wrap items-center gap-2">
                  <h4 className="text-xs font-semibold text-fg">Costliest decisions</h4>
                  <Button size="sm" variant={browsing ? "ghost" : "secondary"} onClick={() => setBrowsing(!browsing)}>
                    {browsing ? "Close" : `Flick through them (${l.costliest.length})`}
                  </Button>
                </div>
                {browsing ? (
                  <FindingBrowser
                    examples={l.costliest.map((c, i) => ({
                      record: i,
                      hand_no: c.hand_no,
                      street: c.street,
                      session_id: c.session_id,
                      text: `${c.category.toLowerCase()}: you chose ${c.taken}, the solver's best was ${c.best} (${c.ev_loss_bb.toFixed(2)}bb, ${c.ev_loss_pct_pot.toFixed(1)}% of the pot)`,
                    }))}
                  />
                ) : (
                  <ul className="flex flex-col gap-0.5 text-xs text-muted" aria-label="Costliest decisions">
                    {l.costliest.slice(0, 5).map((c) => (
                      <li key={`${c.session_id}-${c.hand_no}-${c.decision}`}>
                        <span className="text-fg">
                          Hand {c.hand_no}, {c.street}
                        </span>{" "}
                        ({c.position}): {c.taken}, best {c.best} · {c.ev_loss_bb.toFixed(2)}bb
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
          </>
        )}
      </CardBody>
    </Card>
  );
}

"use client";

import { CircleAlert, CircleCheck } from "lucide-react";
import type { SolverResult } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBb } from "@/lib/units";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { CardRow } from "@/components/poker/CardText";
import { StrategyMatrix } from "./StrategyMatrix";

const bb = (mbb: number) => `${mbb >= 0 ? "+" : "−"}${formatBb(Math.round(Math.abs(mbb)))}bb`;

function Metric({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="rounded-md border border-line bg-surface-2/60 px-3 py-2">
      <p className="label-caps">{label}</p>
      <p className="mt-0.5 font-mono text-sm text-fg tabular">{value}</p>
      {sub && <p className="text-xs text-subtle">{sub}</p>}
    </div>
  );
}

export function ResultView({ jobId }: { jobId: string }) {
  const { data: r, error, loading } = useApi<SolverResult>(`/api/v1/solver/jobs/${jobId}/result`);
  if (error) return <ErrorState message={error.message} />;
  if (loading || !r) return <LoadingState label="Loading result…" />;
  const check = r.independent_check;
  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-muted">
        An approximate equilibrium for the configured bet sizes, produced by GTOpen ({r.algorithm}). It is exploitable
        by at most the figure below; it is not a full-game solution.
      </p>
      <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
        <Metric label="OOP value" value={bb(r.values_mbb[0])} sub="per hand, net of half the pot" />
        <Metric label="IP value" value={bb(r.values_mbb[1])} />
        <Metric
          label="Exploitability (GTOpen)"
          value={`${r.exploitability_pct_pot.toFixed(4)}% pot`}
          sub={`${r.exploitability_mbb.toFixed(2)} mbb · ${r.iterations.toLocaleString()} it · ${r.stop_reason.replaceAll("_", " ")}`}
        />
        <Metric
          label="Independent check"
          value={
            check.performed && check.exploitability_mbb !== null
              ? `${check.exploitability_mbb.toFixed(2)} mbb`
              : "not run"
          }
          sub={check.note}
        />
      </div>
      {check.performed && check.exploitability_mbb !== null && (
        <p className="flex items-center gap-2 text-xs text-accent-strong">
          <CircleCheck aria-hidden className="size-3.5" /> RiverLab&apos;s own best-response calculation reproduced the
          exploitability within {Math.abs(check.exploitability_mbb - r.exploitability_mbb).toFixed(3)} mbb.
        </p>
      )}
      {r.approximations.length > 0 && (
        <ul className="flex flex-col gap-1">
          {r.approximations.map((a) => (
            <li key={a} className="flex items-start gap-2 text-xs text-warn">
              <CircleAlert aria-hidden className="mt-px size-3.5 shrink-0" />
              {a}
            </li>
          ))}
        </ul>
      )}
      {r.root && (
        <div className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-3">
            <h3 className="text-sm font-semibold text-fg">
              First decision — {r.root.player === 0 ? "out of position" : "in position"}, {r.root.street}
            </h3>
            <CardRow cards={r.root.board} size="sm" />
          </div>
          <StrategyMatrix actions={r.root.actions} byClass={r.root.by_class} />
        </div>
      )}
      <p className="font-mono text-[11px] text-subtle">
        {r.solver_backend}@{r.solver_version.slice(0, 10)} · engine {r.engine_version} ·{" "}
        {(r.memory_bytes / 1048576).toFixed(1)} MB · {(r.elapsed_ms / 1000).toFixed(1)} s
      </p>
    </div>
  );
}

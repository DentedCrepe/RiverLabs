"use client";

import { CircleAlert, CircleCheck, Lock } from "lucide-react";
import { api } from "@/lib/api";
import { withLock } from "@/lib/solve";
import type { SolverJob, SolverResult } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { GLOSSARY } from "@/lib/glossary";
import { formatBb } from "@/lib/units";
import { InfoTip } from "@/components/ui/InfoTip";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { CardRow } from "@/components/poker/CardText";
import { type Follow, NodeBrowser } from "./NodeBrowser";
import { StrategyMatrix } from "./StrategyMatrix";

const bb = (mbb: number) => `${mbb >= 0 ? "+" : "−"}${formatBb(Math.round(Math.abs(mbb)))}bb`;

export function Metric({ label, value, sub, info }: { label: string; value: string; sub?: string; info?: string }) {
  return (
    <div className="rounded-md border border-line bg-surface-2/60 px-3 py-2">
      <p className="label-caps flex items-center gap-1">
        {label}
        {info && <InfoTip label={label}>{info}</InfoTip>}
      </p>
      <p className="mt-0.5 font-mono text-sm text-fg tabular">{value}</p>
      {sub && <p className="text-xs text-subtle">{sub}</p>}
    </div>
  );
}

/** A node-locked result: what it is, and how the values moved from the unlocked solve. */
function LockedBanner({ r }: { r: SolverResult }) {
  const base = r.unlocked;
  return (
    <div className="flex flex-col gap-2 rounded-md border border-warn/40 bg-warn-dim px-3 py-2 text-sm">
      <p className="flex items-start gap-2 text-warn">
        <Lock aria-hidden className="mt-0.5 size-4 shrink-0" />
        <span>
          <strong className="font-semibold">Node-locked: an exploit, not an equilibrium.</strong>{" "}
          {r.locks.length === 1 ? "One decision is" : `${r.locks.length} decisions are`} fixed at the frequencies you
          set (marked in the tree below), and everything else is solved as the best answer to them. Exploitability here
          counts only what a player could gain at decisions that are not locked.
        </span>
      </p>
      {base && (
        <p className="font-mono text-xs text-fg tabular">
          Value per hand, unlocked → locked: OOP {bb(base.values_mbb[0])} → {bb(r.values_mbb[0])} · IP{" "}
          {bb(base.values_mbb[1])} → {bb(r.values_mbb[1])}
        </p>
      )}
    </div>
  );
}

export function ResultView({
  jobId,
  follow,
  onLocked,
}: {
  jobId: string;
  follow?: Follow;
  /** Called with the queued job after "Solve with this lock". Locking is offered when given. */
  onLocked?: (job: SolverJob) => void;
}) {
  const { data: r, error, loading } = useApi<SolverResult>(`/api/v1/solver/jobs/${jobId}/result`);
  if (error) return <ErrorState message={error.message} />;
  if (loading || !r) return <LoadingState label="Loading result…" />;
  const check = r.independent_check;
  // The locked banner already says what a node-locked solve is.
  const notes = r.locks.length > 0 ? r.approximations.filter((a) => !a.startsWith("Node-locked")) : r.approximations;
  const lock =
    onLocked &&
    (async (path: number[], frequencies: number[]) => {
      onLocked(await api.post<SolverJob>("/api/v1/solver/jobs", { spec: withLock(r.spec, path, frequencies) }));
    });
  return (
    <div className="flex flex-col gap-4">
      {r.locks.length > 0 ? (
        <LockedBanner r={r} />
      ) : (
        <p className="text-sm text-muted">
          An approximate equilibrium for the configured bet sizes, produced by GTOpen ({r.algorithm}). It is exploitable
          by at most the figure below; it is not a full-game solution.
        </p>
      )}
      <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
        <Metric
          label="OOP value"
          value={bb(r.values_mbb[0])}
          sub="per hand, net of half the pot"
          info={GLOSSARY.solverValue}
        />
        <Metric label="IP value" value={bb(r.values_mbb[1])} info={GLOSSARY.solverValue} />
        <Metric
          label="Exploitability (GTOpen)"
          info={GLOSSARY.exploitability}
          value={`${r.exploitability_pct_pot.toFixed(4)}% pot`}
          sub={`${r.exploitability_mbb.toFixed(2)} mbb · ${r.iterations.toLocaleString()} it · ${r.stop_reason.replaceAll("_", " ")}`}
        />
        <Metric
          label="Independent check"
          info={GLOSSARY.independentCheck}
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
      {notes.length > 0 && (
        <ul className="flex flex-col gap-1">
          {notes.map((a) => (
            <li key={a} className="flex items-start gap-2 text-xs text-warn">
              <CircleAlert aria-hidden className="mt-px size-3.5 shrink-0" />
              {a}
            </li>
          ))}
        </ul>
      )}
      {r.tree_available ? (
        <div className="flex flex-col gap-2">
          <h3 className="text-sm font-semibold text-fg">Strategy tree</h3>
          <NodeBrowser jobId={jobId} follow={follow} locks={r.locks} onLock={lock || undefined} />
        </div>
      ) : (
        r.root && (
          <div className="flex flex-col gap-2">
            <div className="flex flex-wrap items-center gap-3">
              <h3 className="text-sm font-semibold text-fg">
                First decision — {r.root.player === 0 ? "out of position" : "in position"}, {r.root.street}
              </h3>
              <CardRow cards={r.root.board} size="sm" />
            </div>
            <StrategyMatrix actions={r.root.actions} byClass={r.root.by_class} />
          </div>
        )
      )}
      <p className="font-mono text-[11px] text-subtle">
        {r.solver_backend}@{r.solver_version.slice(0, 10)} · engine {r.engine_version} ·{" "}
        {(r.memory_bytes / 1048576).toFixed(1)} MB · {(r.elapsed_ms / 1000).toFixed(1)} s
      </p>
    </div>
  );
}

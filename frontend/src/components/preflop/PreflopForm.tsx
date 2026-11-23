"use client";

import { Play } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ApiError, api } from "@/lib/api";
import { DEFAULT_PREFLOP_FORM, type PreflopForm as Form, buildPreflopSpec, pushFoldForm } from "@/lib/preflop";
import type { SavedGameConfig, SolverJob } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBb, formatBps } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { Field, Select, TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";

export function PreflopForm({ onSubmitted }: { onSubmitted: (job: SolverJob) => void }) {
  const configs = useApi<SavedGameConfig[]>("/api/v1/game-configs");
  const [form, setForm] = useState<Form>(DEFAULT_PREFLOP_FORM);
  const [configId, setConfigId] = useState<string>("");
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [apiError, setApiError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const selected =
    configs.data?.find((c) => c.id === configId) ?? configs.data?.find((c) => c.builtin) ?? configs.data?.[0];
  const set = (patch: Partial<Form>) => setForm((f) => ({ ...f, ...patch }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    setApiError(null);
    if (!selected) {
      setApiError("No game configuration is available");
      return;
    }
    const r = buildPreflopSpec(form, selected.config);
    if (!r.ok) {
      setErrors(r.errors);
      return;
    }
    setErrors({});
    setBusy(true);
    try {
      onSubmitted(await api.post<SolverJob>("/api/v1/solver/preflop/jobs", { spec: r.spec }));
    } catch (err) {
      setApiError(err instanceof ApiError ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const text = (label: string, key: keyof Form, hint?: string) => (
    <Field label={label} hint={hint} error={errors[key as string]}>
      {(id, d) => (
        <TextInput
          id={id}
          aria-describedby={d}
          value={form[key] as string}
          onChange={(e) => set({ [key]: e.target.value } as Partial<Form>)}
        />
      )}
    </Field>
  );
  const check = (label: string, key: "addAllIn" | "allowLimp") => (
    <label className="inline-flex items-center gap-2 text-sm text-fg">
      <input
        type="checkbox"
        className="size-4 accent-[var(--color-accent)]"
        checked={form[key]}
        onChange={(e) => set({ [key]: e.target.checked })}
      />
      {label}
    </label>
  );

  return (
    <form onSubmit={submit} className="flex flex-col gap-5" aria-label="New preflop solve">
      <div className="flex flex-wrap items-center gap-2 text-xs text-muted">
        Presets:
        <Button size="sm" type="button" onClick={() => setForm(pushFoldForm("10"))}>
          Heads-up push/fold 10bb (validated)
        </Button>
        <Button size="sm" type="button" onClick={() => setForm(DEFAULT_PREFLOP_FORM)}>
          Six-max open/3-bet (approximate)
        </Button>
      </div>
      <div className="grid gap-3 sm:grid-cols-3">
        <Field label="Players">
          {(id) => (
            <Select id={id} value={form.players} onChange={(e) => set({ players: e.target.value })}>
              <option value="2">Heads-up (SB vs BB)</option>
              <option value="3">3 (BTN, SB, BB)</option>
              <option value="4">4 (CO–BB)</option>
              <option value="5">5 (HJ–BB)</option>
              <option value="6">6 (UTG–BB)</option>
            </Select>
          )}
        </Field>
        {text("Stack (bb)", "stackBb")}
        <Field label="Game configuration (blinds, ante, rake)">
          {(id) => (
            <Select id={id} value={selected?.id ?? ""} onChange={(e) => setConfigId(e.target.value)}>
              {configs.data?.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name} — {formatBps(c.config.rake.percentage_bps)}% rake
                </option>
              ))}
            </Select>
          )}
        </Field>
        {text("Open-raise sizes (bb, raise to)", "opens", "Blank = all-in or fold only")}
        {text("Re-raise sizes (× the bet faced)", "reraises")}
        {text("Max raises in total", "maxRaises", "2 = open and 3-bet")}
        {text("All-in above % of stack", "allInThresholdPct")}
        {text("Max iterations", "iterations")}
        {text("Time limit (minutes)", "maxMinutes", "Blank = no limit")}
        <Field label="Flop pots are valued with">
          {(id) => (
            <Select
              id={id}
              value={form.realization}
              onChange={(e) => set({ realization: e.target.value as Form["realization"] })}
            >
              <option value="positional">Positional realisation (GTOpen default)</option>
              <option value="full">Full equity realisation</option>
            </Select>
          )}
        </Field>
      </div>
      <div className="flex flex-wrap gap-5">
        {check("Always offer all-in", "addAllIn")}
        {check("Allow limping", "allowLimp")}
      </div>
      <p className="text-xs text-subtle">
        Large trees are slow: six players with more than two raises can take hours on a desktop CPU. Start small.
        {selected &&
          ` Rake from “${selected.name}”: ${formatBps(selected.config.rake.percentage_bps)}%` +
            (selected.config.rake.cap?.unit === "big_blinds"
              ? `, cap ${formatBb(selected.config.rake.cap.mbb)}bb`
              : "") +
            (selected.config.rake.no_flop_no_drop ? ", no flop no drop." : ".")}
      </p>
      {Object.keys(errors).length > 0 && (
        <ErrorState title="Check the highlighted fields" message={Object.values(errors).join(" · ")} />
      )}
      {apiError && <ErrorState title="The solve was not accepted" message={apiError} />}
      <div>
        <Button type="submit" variant="primary" loading={busy} icon={<Play className="size-4" />}>
          Queue preflop solve
        </Button>
      </div>
    </form>
  );
}

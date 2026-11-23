"use client";

import { Play } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ApiError, api } from "@/lib/api";
import { DEFAULT_FORM, STREETS, type SolveForm as Form, type Street, buildSolveSpec } from "@/lib/solve";
import type { GameConfig, SavedGameConfig, SolverJob } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBb, formatBps } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { Field, Select, TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";

function SizeGrid({
  who,
  form,
  errors,
  onChange,
}: {
  who: "oop" | "ip";
  form: Form;
  errors: Record<string, string>;
  onChange: (kind: "bet" | "raise", street: Street, value: string) => void;
}) {
  return (
    <fieldset className="grid grid-cols-[auto_1fr_1fr_1fr] items-center gap-x-2 gap-y-1.5">
      <legend className="label-caps mb-2">{who === "oop" ? "Out of position" : "In position"} — % of pot</legend>
      <span />
      {STREETS.map((s) => (
        <span key={s} className="text-xs text-muted capitalize">
          {s}
        </span>
      ))}
      {(["bet", "raise"] as const).map((kind) => (
        <div key={kind} className="contents">
          <span className="text-xs text-muted">{kind === "bet" ? "Bets" : "Raises"}</span>
          {STREETS.map((s) => {
            const key = `${who}.${kind}.${s}`;
            return (
              <TextInput
                key={key}
                aria-label={`${who.toUpperCase()} ${kind} sizes on the ${s}`}
                aria-invalid={Boolean(errors[key])}
                title={errors[key]}
                className={errors[key] ? "border-danger" : undefined}
                value={form[who][kind][s]}
                placeholder="—"
                onChange={(e) => onChange(kind, s, e.target.value)}
              />
            );
          })}
        </div>
      ))}
    </fieldset>
  );
}

/**
 * `initial` pre-fills the form (e.g. a spot from a hand you played); `game` then fixes the game
 * configuration (that table's rake) instead of offering the saved ones.
 */
export function SolveForm({
  onSubmitted,
  initial,
  game,
}: {
  onSubmitted: (job: SolverJob) => void;
  initial?: Form;
  game?: GameConfig;
}) {
  const configs = useApi<SavedGameConfig[]>("/api/v1/game-configs");
  const [form, setForm] = useState<Form>(initial ?? DEFAULT_FORM);
  const [configId, setConfigId] = useState<string>("");
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [apiError, setApiError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const saved =
    configs.data?.find((c) => c.id === configId) ?? configs.data?.find((c) => c.builtin) ?? configs.data?.[0];
  const selected = game ? { id: "table", name: "your table", config: game } : saved;

  const set = (patch: Partial<Form>) => setForm((f) => ({ ...f, ...patch }));
  const setSize = (who: "oop" | "ip") => (kind: "bet" | "raise", street: Street, value: string) =>
    setForm((f) => ({ ...f, [who]: { ...f[who], [kind]: { ...f[who][kind], [street]: value } } }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    setApiError(null);
    if (!selected) {
      setApiError("No game configuration is available");
      return;
    }
    const r = buildSolveSpec(form, selected.config);
    if (!r.ok) {
      setErrors(r.errors);
      return;
    }
    setErrors({});
    setBusy(true);
    try {
      onSubmitted(await api.post<SolverJob>("/api/v1/solver/jobs", { spec: r.spec }));
    } catch (err) {
      setApiError(err instanceof ApiError ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const text = (label: string, key: keyof Form, hint?: string, wide = false) => (
    <div className={wide ? "sm:col-span-2" : undefined}>
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
    </div>
  );

  return (
    <form onSubmit={submit} className="flex flex-col gap-5" aria-label="New postflop solve">
      <div className="grid gap-3 sm:grid-cols-2">
        {game ? (
          <Field label="Game configuration (rake)">
            {(id) => <TextInput id={id} readOnly value={`Your table — ${formatBps(game.rake.percentage_bps)}% rake`} />}
          </Field>
        ) : (
          <Field label="Game configuration (rake)">
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
        )}
        {text("Board (flop, turn or river)", "board")}
        {text("OOP range", "oopRange", undefined, true)}
        {text("IP range", "ipRange", undefined, true)}
        {text("Starting pot (bb)", "potBb")}
        {text("Effective stack (bb)", "stackBb")}
      </div>
      <div className="grid gap-5 lg:grid-cols-2">
        <SizeGrid who="oop" form={form} errors={errors} onChange={setSize("oop")} />
        <SizeGrid who="ip" form={form} errors={errors} onChange={setSize("ip")} />
      </div>
      <div className="grid gap-3 sm:grid-cols-4">
        {text("Max raises / street", "maxRaises")}
        {text("All-in at % of stack", "allInThresholdPct", "100 = exact only")}
        {text("Max iterations", "iterations")}
        {text("Stop at exploitability (% pot)", "targetPct", "Blank = run all iterations")}
      </div>
      <label className="inline-flex items-center gap-2 text-sm text-fg">
        <input
          type="checkbox"
          className="size-4 accent-[var(--color-accent)]"
          checked={form.addAllIn}
          onChange={(e) => set({ addAllIn: e.target.checked })}
        />
        Always offer all-in
      </label>
      {selected && (
        <p className="text-xs text-subtle">
          Rake from “{selected.name}”: {formatBps(selected.config.rake.percentage_bps)}%
          {selected.config.rake.cap?.unit === "big_blinds" ? `, cap ${formatBb(selected.config.rake.cap.mbb)}bb` : ""}.
          Heads-up postflop only; OOP leads into the previous street&apos;s bettor use its normal bet sizes.
        </p>
      )}
      {Object.keys(errors).length > 0 && (
        <ErrorState title="Check the highlighted fields" message={Object.values(errors).join(" · ")} />
      )}
      {apiError && <ErrorState title="The solve was not accepted" message={apiError} />}
      <div>
        <Button type="submit" variant="primary" loading={busy} icon={<Play className="size-4" />}>
          Queue solve
        </Button>
      </div>
    </form>
  );
}

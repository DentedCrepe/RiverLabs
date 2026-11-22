"use client";

import { CircleCheck, Save, Trash2 } from "lucide-react";
import { type ReactNode, useState } from "react";
import { ApiError, api } from "@/lib/api";
import type { GameConfig, RakeResult, SavedGameConfig, StackPresets } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { bbToMbb, formatBb, formatBps } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, Select, TextInput } from "@/components/ui/Field";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { type AmountForm, type ConfigForm, configToForm, formToConfig } from "./configForm";

function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label className="inline-flex cursor-pointer items-center gap-2 text-sm text-fg">
      <input
        type="checkbox"
        className="size-4 accent-[var(--color-accent)]"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      {label}
    </label>
  );
}

function AmountInput({
  label,
  value,
  onChange,
  error,
  optional = true,
}: {
  label: string;
  value: AmountForm;
  onChange: (v: AmountForm) => void;
  error?: string;
  optional?: boolean;
}) {
  return (
    <Field label={label} error={error}>
      {(id, d) => (
        <div className="flex flex-col gap-1.5">
          {optional && (
            <Toggle checked={value.enabled} onChange={(enabled) => onChange({ ...value, enabled })} label="Enabled" />
          )}
          <div className="flex gap-1.5">
            <TextInput
              id={id}
              aria-describedby={d}
              disabled={optional && !value.enabled}
              value={value.value}
              onChange={(e) => onChange({ ...value, value: e.target.value })}
            />
            <Select
              aria-label={`${label} unit`}
              className="w-28"
              disabled={optional && !value.enabled}
              value={value.unit}
              onChange={(e) => onChange({ ...value, unit: e.target.value as AmountForm["unit"] })}
            >
              <option value="big_blinds">bb</option>
              <option value="currency">minor units</option>
            </Select>
          </div>
        </div>
      )}
    </Field>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="grid gap-4 border-t border-line pt-4 first:border-t-0 first:pt-0 sm:grid-cols-2 lg:grid-cols-3">
      <legend className="label-caps mb-3 sm:col-span-full">{title}</legend>
      {children}
    </fieldset>
  );
}

export function GameConfigEditor() {
  const defaults = useApi<GameConfig>("/api/v1/game-configs/default");
  if (defaults.error)
    return <ErrorState title="Could not load the default configuration" message={defaults.error.message} />;
  if (!defaults.data) return <LoadingState label="Loading configuration…" />;
  return <Editor initial={defaults.data} />;
}

function Editor({ initial }: { initial: GameConfig }) {
  const presets = useApi<StackPresets>("/api/v1/stack-presets");
  const saved = useApi<SavedGameConfig[]>("/api/v1/game-configs");
  const [form, setForm] = useState<ConfigForm>(() => configToForm(initial));
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [engineError, setEngineError] = useState<string | null>(null);
  const [validated, setValidated] = useState<GameConfig | null>(null);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState<"validate" | "save" | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [potBb, setPotBb] = useState("20");
  const [sawFlop, setSawFlop] = useState(true);
  const [rake, setRake] = useState<RakeResult | null>(null);

  const update = (patch: Partial<ConfigForm>) => {
    setForm({ ...form, ...patch });
    setValidated(null);
    setRake(null);
    setNotice(null);
  };

  function convert(): GameConfig | null {
    const r = formToConfig(form);
    if (!r.ok) {
      setFieldErrors(r.errors);
      return null;
    }
    setFieldErrors({});
    return r.config;
  }

  async function validate() {
    const cfg = convert();
    if (!cfg) return;
    setBusy("validate");
    setEngineError(null);
    try {
      const canonical = await api.post<GameConfig>("/api/v1/game-configs/validate", cfg);
      setValidated(canonical);
      const mbb = bbToMbb(potBb);
      if (mbb !== null) {
        setRake(
          await api.post<RakeResult>("/api/v1/rake/calculate", { config: canonical, pot_mbb: mbb, saw_flop: sawFlop }),
        );
      }
    } catch (e) {
      setValidated(null);
      setEngineError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  }

  async function save() {
    const cfg = convert();
    if (!cfg || !name.trim()) return;
    setBusy("save");
    setEngineError(null);
    try {
      await api.post<SavedGameConfig>("/api/v1/game-configs", { name: name.trim(), config: cfg });
      setNotice(`Saved “${name.trim()}”.`);
      setName("");
      saved.reload();
    } catch (e) {
      setEngineError(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  }

  async function remove(id: string) {
    try {
      await api.delete(`/api/v1/game-configs/${id}`);
      saved.reload();
    } catch (e) {
      setEngineError(e instanceof ApiError ? e.message : String(e));
    }
  }

  const currentStackMbb = bbToMbb(form.stackBb);

  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
      <Card>
        <CardHeader
          title="Game configuration"
          subtitle="No-limit hold'em cash game. Stack depth and rake are configured independently."
        />
        <CardBody className="flex flex-col gap-6">
          <Section title="Table">
            <Field label="Players" error={fieldErrors.players} hint="3–6 (heads-up is not supported yet)">
              {(id, d) => (
                <TextInput
                  id={id}
                  aria-describedby={d}
                  value={form.players}
                  onChange={(e) => update({ players: e.target.value })}
                />
              )}
            </Field>
            <Field label="Small blind (bb)" error={fieldErrors.smallBlindBb}>
              {(id, d) => (
                <TextInput
                  id={id}
                  aria-describedby={d}
                  value={form.smallBlindBb}
                  onChange={(e) => update({ smallBlindBb: e.target.value })}
                />
              )}
            </Field>
            <Field label="Ante per player (bb)" error={fieldErrors.anteBb}>
              {(id, d) => (
                <TextInput
                  id={id}
                  aria-describedby={d}
                  value={form.anteBb}
                  onChange={(e) => update({ anteBb: e.target.value })}
                />
              )}
            </Field>
          </Section>

          <Section title="Effective stack">
            <div className="flex flex-wrap gap-1.5 sm:col-span-2" role="radiogroup" aria-label="Stack presets">
              {presets.data?.presets.map((p) => {
                const active = currentStackMbb === p.mbb;
                return (
                  <button
                    key={p.label}
                    type="button"
                    role="radio"
                    aria-checked={active}
                    onClick={() => update({ stackBb: String(p.big_blinds) })}
                    className={cn(
                      "h-8 rounded-md border px-3 font-mono text-sm",
                      active
                        ? "border-accent bg-accent-dim text-accent-strong"
                        : "border-line text-muted hover:text-fg",
                    )}
                  >
                    {p.label}
                  </button>
                );
              })}
            </div>
            <Field
              label="Custom depth (bb)"
              error={fieldErrors.stackBb}
              hint={
                presets.data
                  ? `${formatBb(presets.data.min_mbb)}–${formatBb(presets.data.max_mbb)}bb, 0.001bb resolution`
                  : undefined
              }
            >
              {(id, d) => (
                <TextInput
                  id={id}
                  aria-describedby={d}
                  value={form.stackBb}
                  onChange={(e) => update({ stackBb: e.target.value })}
                />
              )}
            </Field>
          </Section>

          <Section title="Rake">
            <Field label="Percentage (%)" error={fieldErrors.rakePercent}>
              {(id, d) => (
                <TextInput
                  id={id}
                  aria-describedby={d}
                  value={form.rakePercent}
                  onChange={(e) => update({ rakePercent: e.target.value })}
                />
              )}
            </Field>
            <AmountInput label="Cap" value={form.cap} error={fieldErrors.cap} onChange={(cap) => update({ cap })} />
            <AmountInput
              label="Minimum raked pot"
              value={form.minPot}
              error={fieldErrors.minPot}
              onChange={(minPot) => update({ minPot })}
            />
            <AmountInput
              label="Rounding increment"
              optional={false}
              value={{ enabled: true, unit: form.roundingUnit, value: form.roundingValue }}
              error={fieldErrors.rounding}
              onChange={(v) => update({ roundingUnit: v.unit, roundingValue: v.value })}
            />
            <Field label="Rounding mode">
              {(id) => (
                <Select
                  id={id}
                  value={form.roundingMode}
                  onChange={(e) => update({ roundingMode: e.target.value as ConfigForm["roundingMode"] })}
                >
                  <option value="down">Down (truncate)</option>
                  <option value="nearest">Nearest (half up)</option>
                  <option value="up">Up</option>
                </Select>
              )}
            </Field>
            <Field label="Side-pot allocation">
              {(id) => (
                <Select
                  id={id}
                  value={form.allocation}
                  onChange={(e) => update({ allocation: e.target.value as ConfigForm["allocation"] })}
                >
                  <option value="proportional">Proportional to pot size</option>
                  <option value="main_pot_first">Main pot first</option>
                </Select>
              )}
            </Field>
            <div className="sm:col-span-full">
              <Toggle
                checked={form.noFlopNoDrop}
                onChange={(noFlopNoDrop) => update({ noFlopNoDrop })}
                label="No flop, no drop"
              />
            </div>
          </Section>

          <Section title="Currency stake (needed for currency rake amounts)">
            <div className="sm:col-span-full">
              <Toggle
                checked={form.stakeEnabled}
                onChange={(stakeEnabled) => update({ stakeEnabled })}
                label="Define a currency stake"
              />
            </div>
            <Field label="Currency (ISO 4217)">
              {(id) => (
                <TextInput
                  id={id}
                  disabled={!form.stakeEnabled}
                  value={form.stakeCurrency}
                  onChange={(e) => update({ stakeCurrency: e.target.value })}
                />
              )}
            </Field>
            <Field label="Big blind (minor units)" error={fieldErrors.stake} hint="e.g. 100 = $1.00">
              {(id, d) => (
                <TextInput
                  id={id}
                  aria-describedby={d}
                  disabled={!form.stakeEnabled}
                  value={form.stakeBigBlindMinor}
                  onChange={(e) => update({ stakeBigBlindMinor: e.target.value })}
                />
              )}
            </Field>
          </Section>

          {engineError && <ErrorState title="The engine rejected this configuration" message={engineError} />}
          {validated && (
            <p role="status" className="flex items-center gap-2 text-sm text-accent-strong">
              <CircleCheck aria-hidden className="size-4" /> Valid: {validated.players}-handed,{" "}
              {formatBb(validated.stack.effective_stack)}bb, {formatBps(validated.rake.percentage_bps)}% rake.
            </p>
          )}
          {notice && (
            <p role="status" className="text-sm text-accent-strong">
              {notice}
            </p>
          )}
          <div className="flex flex-wrap items-end gap-3 border-t border-line pt-4">
            <Button variant="primary" onClick={validate} loading={busy === "validate"}>
              Validate &amp; preview rake
            </Button>
            <div className="flex items-end gap-2">
              <Field label="Save as">
                {(id) => (
                  <TextInput
                    id={id}
                    className="w-52 font-sans"
                    placeholder="Configuration name"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                  />
                )}
              </Field>
              <Button
                onClick={save}
                loading={busy === "save"}
                disabled={!name.trim()}
                icon={<Save className="size-4" />}
              >
                Save
              </Button>
            </div>
          </div>
        </CardBody>
      </Card>

      <div className="flex flex-col gap-6">
        <Card>
          <CardHeader title="Rake preview" subtitle="Engine calculation for one final pot" />
          <CardBody className="flex flex-col gap-3">
            <div className="grid grid-cols-2 gap-3">
              <Field label="Final pot (bb)">
                {(id) => (
                  <TextInput
                    id={id}
                    value={potBb}
                    onChange={(e) => {
                      setPotBb(e.target.value);
                      setRake(null);
                    }}
                  />
                )}
              </Field>
              <div className="flex items-end pb-1.5">
                <Toggle
                  checked={sawFlop}
                  onChange={(v) => {
                    setSawFlop(v);
                    setRake(null);
                  }}
                  label="Flop dealt"
                />
              </div>
            </div>
            {rake ? (
              <dl className="grid grid-cols-2 gap-y-2 font-mono text-sm tabular">
                <dt className="label-caps self-center font-sans">Rake</dt>
                <dd className="text-right text-fg">{formatBb(rake.rake)}bb</dd>
                <dt className="label-caps self-center font-sans">Rule applied</dt>
                <dd className="text-right text-muted">{rake.reason.replaceAll("_", " ")}</dd>
              </dl>
            ) : (
              <p className="text-xs text-subtle">Press “Validate &amp; preview rake” to compute.</p>
            )}
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Saved configurations" />
          <CardBody>
            {saved.loading && !saved.data ? (
              <LoadingState />
            ) : saved.error ? (
              <ErrorState message={saved.error.message} />
            ) : saved.data && saved.data.length > 0 ? (
              <ul className="divide-y divide-line">
                {saved.data.map((c) => (
                  <li key={c.id} className="flex items-center gap-2 py-2">
                    <button
                      type="button"
                      className="min-w-0 flex-1 text-left"
                      onClick={() => update(configToForm(c.config))}
                    >
                      <span className="block truncate text-sm text-fg">{c.name}</span>
                      <span className="block font-mono text-xs text-muted">
                        {c.config.players}-max · {formatBb(c.config.stack.effective_stack)}bb ·{" "}
                        {formatBps(c.config.rake.percentage_bps)}%{c.config.rake.no_flop_no_drop ? " NFND" : ""}
                      </span>
                    </button>
                    {c.builtin ? (
                      <span className="text-[10px] tracking-wide text-subtle uppercase">Built-in</span>
                    ) : (
                      <Button
                        size="sm"
                        variant="ghost"
                        aria-label={`Delete ${c.name}`}
                        onClick={() => remove(c.id)}
                        icon={<Trash2 className="size-3.5" />}
                      />
                    )}
                  </li>
                ))}
              </ul>
            ) : (
              <EmptyState title="No saved configurations" description="Saved configurations appear here." />
            )}
          </CardBody>
        </Card>
      </div>
    </div>
  );
}

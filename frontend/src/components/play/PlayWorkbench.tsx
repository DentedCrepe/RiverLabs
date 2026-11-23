"use client";

import { Eye, Play, Trash2 } from "lucide-react";
import { type FormEvent, useEffect, useState } from "react";
import { ApiError, api } from "@/lib/api";
import {
  DEFAULT_PLAY_FORM,
  type PlayForm,
  type PlaySessionInfo,
  type PlayState,
  amount,
  buildTableConfig,
} from "@/lib/play";
import type { SavedGameConfig } from "@/lib/types";
import { useApi } from "@/lib/useApi";
import { formatBps } from "@/lib/units";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, Select, TextInput } from "@/components/ui/Field";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { PlayReport } from "./PlayReport";
import { PlayTable } from "./PlayTable";

const FORMAT_LABEL: Record<PlaySessionInfo["format"], string> = {
  cash: "Cash",
  fast_fold: "Fast-fold",
  sit_and_go: "Sit & go",
};

function Setup({ onCreated }: { onCreated: (s: PlayState) => void }) {
  const configs = useApi<SavedGameConfig[]>("/api/v1/game-configs");
  const [form, setForm] = useState<PlayForm>(DEFAULT_PLAY_FORM);
  const [configId, setConfigId] = useState("");
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [apiError, setApiError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const selected =
    configs.data?.find((c) => c.id === configId) ?? configs.data?.find((c) => c.builtin) ?? configs.data?.[0];
  const set = (patch: Partial<PlayForm>) => setForm((f) => ({ ...f, ...patch }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    setApiError(null);
    if (!selected) return setApiError("No game configuration is available");
    const r = buildTableConfig(form, selected.config, Math.floor(Math.random() * 2 ** 31));
    if (!r.ok) return setErrors(r.errors);
    setErrors({});
    setBusy(true);
    try {
      onCreated(await api.post<PlayState>("/api/v1/play/sessions", { config: r.config }));
    } catch (err) {
      setApiError(err instanceof ApiError ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const text = (label: string, key: keyof PlayForm, hint?: string) => (
    <Field label={label} hint={hint} error={errors[key]}>
      {(id, d) => (
        <TextInput
          id={id}
          aria-describedby={d}
          value={form[key] as string}
          onChange={(e) => set({ [key]: e.target.value } as Partial<PlayForm>)}
        />
      )}
    </Field>
  );
  const sng = form.format === "sit_and_go";
  return (
    <form onSubmit={submit} className="flex flex-col gap-4" aria-label="New game">
      <div className="grid gap-3 sm:grid-cols-3">
        <Field label="Game type">
          {(id) => (
            <Select id={id} value={form.format} onChange={(e) => set({ format: e.target.value as PlayForm["format"] })}>
              <option value="cash">Cash game</option>
              <option value="fast_fold">Fast-fold (new table every hand)</option>
              <option value="sit_and_go">Sit &amp; go (rising blinds)</option>
            </Select>
          )}
        </Field>
        <Field label="Seats (including you)" error={errors.seats}>
          {(id) => (
            <Select id={id} value={form.seats} onChange={(e) => set({ seats: e.target.value })}>
              {[2, 3, 4, 5, 6].map((n) => (
                <option key={n} value={n}>
                  {n === 2 ? "2 (heads-up)" : n}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label="Bot difficulty">
          {(id) => (
            <Select
              id={id}
              value={form.difficulty}
              onChange={(e) => set({ difficulty: e.target.value as PlayForm["difficulty"] })}
            >
              <option value="mixed">Mixed (easy, medium, hard)</option>
              <option value="easy">Easy: calls too much, never bluffs</option>
              <option value="medium">Medium: small leaks</option>
              <option value="hard">Hard: follows its strategy</option>
            </Select>
          )}
        </Field>
        {sng ? (
          <>
            {text("Starting chips", "startingChips")}
            {text("Hands per level", "handsPerLevel")}
            <div className="sm:col-span-3">
              {text("Blind levels (small/big/ante)", "levels", "Comma-separated, e.g. 10/20, 15/30, 25/50/5")}
            </div>
          </>
        ) : (
          <>
            {text("Stack (bb)", "stackBb")}
            <Field label="Blinds, ante and rake">
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
            {form.format === "cash" && (
              <label className="inline-flex items-center gap-2 self-end pb-2 text-sm text-fg">
                <input
                  type="checkbox"
                  className="size-4 accent-[var(--color-accent)]"
                  checked={form.autoRebuy}
                  onChange={(e) => set({ autoRebuy: e.target.checked })}
                />
                Rebuy when I bust
              </label>
            )}
          </>
        )}
      </div>
      <p className="text-xs text-subtle">
        Bots play preflop from GTOpen&apos;s preflop model (an approximate model) and postflop by simple, documented
        rules; they are practice opponents, not solvers. A new table shape takes up to a minute to prepare the first
        time.
      </p>
      {Object.keys(errors).length > 0 && (
        <ErrorState title="Check the highlighted fields" message={Object.values(errors).join(" · ")} />
      )}
      {apiError && <ErrorState title="The game was not created" message={apiError} />}
      <div>
        <Button type="submit" variant="primary" loading={busy} icon={<Play className="size-4" />}>
          Sit down
        </Button>
      </div>
    </form>
  );
}

function Games({
  games,
  onOpen,
  onChanged,
}: {
  games: PlaySessionInfo[];
  onOpen: (id: string) => void;
  onChanged: () => void;
}) {
  if (games.length === 0)
    return <EmptyState title="No games yet" description="Your games and their reports appear here." />;
  return (
    <ul className="divide-y divide-line">
      {games.map((g) => {
        const units = g.format === "sit_and_go" ? "chips" : "bb";
        return (
          <li key={g.id} className="flex flex-wrap items-center gap-3 py-2 text-xs">
            <span className="rounded bg-surface-3 px-1.5 py-px font-medium uppercase">{g.status}</span>
            <span className="text-fg">{FORMAT_LABEL[g.format]}</span>
            <span className="font-mono text-muted tabular">
              {g.hands} hands · {g.hero_net >= 0 ? "+" : "−"}
              {amount(Math.abs(g.hero_net), units)}
            </span>
            <span className="ml-auto flex gap-1">
              <Button size="sm" variant="ghost" onClick={() => onOpen(g.id)} icon={<Eye className="size-3.5" />}>
                {g.status === "active" || g.status === "preparing" ? "Play" : "Report"}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                aria-label="Delete game"
                onClick={async () => {
                  await api.delete(`/api/v1/play/sessions/${g.id}`).catch(() => undefined);
                  onChanged();
                }}
                icon={<Trash2 className="size-3.5" />}
              />
            </span>
          </li>
        );
      })}
    </ul>
  );
}

export function PlayWorkbench() {
  const games = useApi<PlaySessionInfo[]>("/api/v1/play/sessions?limit=20");
  const [state, setState] = useState<PlayState | null>(null);
  const [openError, setOpenError] = useState<string | null>(null);
  const id = state?.session.id;
  const preparing = state?.session.status === "preparing";

  // Poll while the table is being prepared.
  useEffect(() => {
    if (!id || !preparing) return;
    const t = setInterval(async () => {
      try {
        setState(await api.get<PlayState>(`/api/v1/play/sessions/${id}`, { poll: true }));
      } catch (e) {
        setOpenError(e instanceof ApiError ? e.message : String(e));
      }
    }, 1500);
    return () => clearInterval(t);
  }, [id, preparing]);

  async function open(sid: string) {
    setOpenError(null);
    try {
      setState(await api.get<PlayState>(`/api/v1/play/sessions/${sid}`));
    } catch (e) {
      setOpenError(e instanceof ApiError ? e.message : String(e));
    }
  }

  const s = state?.session;
  const playable = s?.status === "active" && state?.table && state.table.status !== "session_over";
  return (
    <div className="grid gap-6">
      {!s && (
        <div className="grid gap-6 xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
          <Card>
            <CardHeader title="New game" subtitle="Cash, fast-fold or sit & go against bots" />
            <CardBody>
              <Setup
                onCreated={(st) => {
                  setState(st);
                  games.reload();
                }}
              />
            </CardBody>
          </Card>
          <Card>
            <CardHeader title="Your games" />
            <CardBody className="py-1">
              {games.error ? (
                <ErrorState message={games.error.message} />
              ) : games.loading && !games.data ? (
                <LoadingState />
              ) : (
                <Games games={games.data ?? []} onOpen={open} onChanged={games.reload} />
              )}
              {openError && <ErrorState message={openError} />}
            </CardBody>
          </Card>
        </div>
      )}
      {s && (
        <>
          <div>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                setState(null);
                games.reload();
              }}
            >
              ← All games
            </Button>
          </div>
          <Card>
            <CardHeader title={`${FORMAT_LABEL[s.format]} table`} subtitle={`Status: ${s.status}`} />
            <CardBody>
              {s.status === "preparing" ? (
                <LoadingState label="Preparing the table: solving the bots' preflop model (up to a minute the first time)…" />
              ) : s.status === "failed" ? (
                <ErrorState title="The table could not be prepared" message={s.error_message ?? "Unknown error"} />
              ) : playable || (s.status === "active" && state.table) ? (
                <PlayTable state={state} onChange={setState} />
              ) : (
                <p className="text-sm text-muted">This game has ended ({s.status}). Its report is below.</p>
              )}
              {openError && <ErrorState message={openError} />}
            </CardBody>
          </Card>
          {s.status !== "preparing" && s.status !== "failed" && (!playable || s.hands > 0) && (
            <Card>
              <CardHeader
                title="Session report"
                subtitle="Statistics and weaknesses, each with what it is measured against"
              />
              <CardBody>
                <PlayReport sessionId={s.id} refreshKey={s.hands} />
              </CardBody>
            </Card>
          )}
        </>
      )}
    </div>
  );
}

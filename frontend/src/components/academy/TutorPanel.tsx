"use client";

import { Bot, Send, TriangleAlert } from "lucide-react";
import Link from "next/link";
import { useState, useSyncExternalStore } from "react";
import { ApiError, api } from "@/lib/api";
import type { ExplanationLevel } from "@/lib/academy";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { Markdown } from "./Markdown";

export type TutorStatus = { available: boolean; reason: string | null; model: string | null };

export type TutorAnswer = {
  answer: string;
  sections: string[];
  readings: string[];
  dropped_readings: string[];
  suggested_lessons: string[];
  tool_calls: { tool: string; input: Record<string, unknown>; ok: boolean; error?: string }[];
  unverified_numbers: string[];
  corrections: number;
  model: string | null;
  label: string;
};

type Turn = { role: "learner"; text: string } | { role: "tutor"; text: string; answer: TutorAnswer };

const CONSENT_KEY = "riverlab.tutorConsent";
const CONSENT_EVENT = "riverlab-tutor-consent";

function readConsent(): boolean {
  try {
    return typeof window !== "undefined" && window.localStorage.getItem(CONSENT_KEY) === "yes";
  } catch {
    return false;
  }
}

function setConsent(yes: boolean) {
  try {
    if (yes) window.localStorage.setItem(CONSENT_KEY, "yes");
    else window.localStorage.removeItem(CONSENT_KEY);
  } catch {
    // not remembered: asked again next time
  }
  window.dispatchEvent(new Event(CONSENT_EVENT));
}

function useConsent(): boolean {
  return useSyncExternalStore(
    (cb) => {
      window.addEventListener(CONSENT_EVENT, cb);
      return () => window.removeEventListener(CONSENT_EVENT, cb);
    },
    readConsent,
    () => false,
  );
}

const TOOL_LABEL: Record<string, string> = {
  theory: "theory formula",
  equity: "equity engine",
  analyze_range: "range engine",
  arithmetic: "arithmetic",
  lesson: "another lesson",
  curriculum: "curriculum",
  list_solves: "your solves",
  solver_result: "a stored solve",
};

function Sources({ a, slug }: { a: TutorAnswer; slug: string }) {
  const used = a.tool_calls.filter((c) => c.ok);
  return (
    <div className="flex flex-col gap-1 text-[11px] text-muted" aria-label="Tutor sources">
      {a.sections.length > 0 && (
        <p>
          Lesson sections:{" "}
          {a.sections.map((s, i) => {
            const [lesson, id] = s.includes("#") ? s.split("#") : [slug, s];
            return (
              <span key={s}>
                {i > 0 && ", "}
                <Link href={`/academy/${lesson}#${id}`} className="text-accent hover:underline">
                  {lesson === slug ? `#${id}` : `${lesson}#${id}`}
                </Link>
              </span>
            );
          })}
        </p>
      )}
      {used.length > 0 && (
        <p>
          Numbers from RiverLab: {[...new Set(used.map((c) => TOOL_LABEL[c.tool] ?? c.tool))].join(", ")} ({used.length}{" "}
          call{used.length === 1 ? "" : "s"})
        </p>
      )}
      {a.readings.length > 0 && <p>Reading: {a.readings.join("; ")}</p>}
      {a.suggested_lessons.length > 0 && (
        <p>
          Read first:{" "}
          {a.suggested_lessons.map((l, i) => (
            <span key={l}>
              {i > 0 && ", "}
              <Link href={`/academy/${l}`} className="text-accent hover:underline">
                {l}
              </Link>
            </span>
          ))}
        </p>
      )}
      {a.dropped_readings.length > 0 && (
        <p>Removed references not in any lesson&apos;s reading list: {a.dropped_readings.join("; ")}</p>
      )}
    </div>
  );
}

/** Ask Claude about this lesson; every number must come from RiverLab (docs/academy-ai-tutor.md). */
export function TutorPanel({ slug, level }: { slug: string; level: ExplanationLevel }) {
  const status = useApi<TutorStatus>("/api/v1/academy/tutor");
  const consent = useConsent();
  const [turns, setTurns] = useState<Turn[]>([]);
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function send() {
    const q = question.trim();
    if (!q) return;
    setBusy(true);
    setError(null);
    const history = turns.map((t) => ({ role: t.role, text: t.text }));
    setTurns((ts) => [...ts, { role: "learner", text: q }]);
    setQuestion("");
    try {
      const a = await api.post<TutorAnswer>(`/api/v1/academy/lessons/${slug}/tutor`, {
        level,
        question: q,
        history,
        consent: true,
      });
      setTurns((ts) => [...ts, { role: "tutor", text: a.answer, answer: a }]);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
      setTurns((ts) => ts.slice(0, -1));
      setQuestion(q);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-label="AI tutor" className="flex flex-col gap-3 rounded-lg border border-line p-4">
      <h2 className="flex items-center gap-2 text-lg font-semibold text-fg">
        <Bot aria-hidden className="size-5 text-accent" />
        Ask the AI tutor
      </h2>
      {status.error ? (
        <ErrorState message={status.error.message} />
      ) : !status.data ? (
        <LoadingState label="Checking the tutor…" />
      ) : !status.data.available ? (
        <p className="text-sm text-muted">
          The AI tutor is not set up on this RiverLab: {status.data.reason}. It uses Anthropic&apos;s API; to turn it
          on, add an API key to RiverLab&apos;s <code>.env</code> (<code>ANTHROPIC_API_KEY</code>) and restart.
        </p>
      ) : !consent ? (
        <div className="flex flex-col gap-2 text-sm text-muted">
          <p>
            The tutor is Claude, an AI model from Anthropic. To answer, RiverLab sends Anthropic this lesson, your
            Academy progress (lessons completed, concepts you got wrong) and your questions. It never sends your hands
            or games. Anthropic&apos;s API is paid by whoever owns the key.
          </p>
          <div>
            <Button size="sm" variant="primary" onClick={() => setConsent(true)}>
              I agree, turn the tutor on
            </Button>
          </div>
        </div>
      ) : (
        <>
          <p className="flex items-start gap-1.5 text-xs text-muted">
            <TriangleAlert aria-hidden className="mt-0.5 size-3.5 shrink-0 text-warn" />
            AI tutor (may be wrong): the lesson stays the reviewed source. Numbers must come from RiverLab&apos;s
            engine, the lesson or this chat; any it cannot trace are marked.
          </p>
          {turns.length > 0 && (
            <ol className="flex flex-col gap-3" aria-label="Tutor conversation">
              {turns.map((t, i) => (
                <li
                  key={i}
                  className={cn(
                    "rounded-md px-3 py-2 text-sm",
                    t.role === "learner" ? "ml-8 bg-surface-2 text-fg" : "mr-4 border border-line",
                  )}
                >
                  {t.role === "learner" ? (
                    <p className="whitespace-pre-wrap">{t.text}</p>
                  ) : (
                    <div className="flex flex-col gap-2">
                      <Markdown text={t.text} />
                      <Sources a={t.answer} slug={slug} />
                    </div>
                  )}
                </li>
              ))}
            </ol>
          )}
          {busy && <LoadingState label="The tutor is working it out (this can take up to a minute)…" />}
          {error && <ErrorState message={error} />}
          <div className="flex flex-col gap-2">
            <label htmlFor="tutor-question" className="sr-only">
              Your question
            </label>
            <textarea
              id="tutor-question"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              rows={2}
              maxLength={4000}
              placeholder="e.g. Why does a bigger bet need more equity to call?"
              className="w-full rounded-md border border-line bg-canvas px-3 py-2 text-sm text-fg"
            />
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="primary"
                onClick={send}
                disabled={busy || !question.trim()}
                icon={<Send className="size-3.5" />}
              >
                Ask
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setConsent(false)}>
                Turn the tutor off
              </Button>
            </div>
          </div>
        </>
      )}
    </section>
  );
}

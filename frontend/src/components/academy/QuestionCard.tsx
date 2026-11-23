"use client";

import { CircleCheck, CircleX } from "lucide-react";
import { type FormEvent, useEffect, useRef, useState } from "react";
import { ApiError, api } from "@/lib/api";
import type { AttemptResult, Question } from "@/lib/academy";
import { Button } from "@/components/ui/Button";
import { TextInput } from "@/components/ui/Field";
import { ErrorState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { Markdown } from "./Markdown";

/** One knowledge-check question. Answers are graded by the API, which holds the answer key. */
export function QuestionCard({
  question,
  onAnswered,
}: {
  question: Question;
  onAnswered?: (r: AttemptResult) => void;
}) {
  const [choice, setChoice] = useState<number | null>(null);
  const [value, setValue] = useState("");
  const [confidence, setConfidence] = useState<number | null>(null);
  const [result, setResult] = useState<AttemptResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  // When the question was shown, for time-spent tracking (set after mount; event handlers reset it).
  const started = useRef(0);
  useEffect(() => {
    started.current = Date.now();
  }, []);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    const answer = question.kind === "numeric" ? Number(value) : choice;
    if (answer === null || (question.kind === "numeric" && (value.trim() === "" || Number.isNaN(answer)))) {
      setError(question.kind === "numeric" ? "Enter a number" : "Choose an answer");
      return;
    }
    try {
      const r = await api.post<AttemptResult>(`/api/v1/academy/questions/${encodeURIComponent(question.id)}/attempts`, {
        answer,
        confidence,
        time_ms: Date.now() - started.current,
      });
      setResult(r);
      onAnswered?.(r);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : String(err));
    }
  }

  function retry() {
    setResult(null);
    setChoice(null);
    setValue("");
    started.current = Date.now();
  }

  return (
    <form
      onSubmit={submit}
      className="flex flex-col gap-2 rounded-md border border-line p-3"
      aria-label="Knowledge check"
    >
      <Markdown text={question.prompt} />
      {question.options ? (
        <div role="radiogroup" className="flex flex-col gap-1">
          {question.options.map((o, i) => (
            <label
              key={i}
              className={cn(
                "flex cursor-pointer items-start gap-2 rounded px-2 py-1 text-sm",
                result && i === result.answer && "bg-accent-dim/60",
                result && choice === i && !result.correct && "bg-danger-dim/60",
              )}
            >
              <input
                type="radio"
                name={question.id}
                checked={choice === i}
                disabled={!!result}
                onChange={() => setChoice(i)}
                className="mt-1"
              />
              <span>{o}</span>
            </label>
          ))}
        </div>
      ) : (
        <label className="flex items-center gap-2 text-sm">
          Answer
          <TextInput
            aria-label="Answer"
            value={value}
            disabled={!!result}
            onChange={(e) => setValue(e.target.value)}
            className="w-28"
          />
          <span className="text-muted">{question.unit}</span>
        </label>
      )}
      {!result && (
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted">
          How sure are you?
          {[1, 2, 3, 4, 5].map((c) => (
            <button
              key={c}
              type="button"
              aria-pressed={confidence === c}
              onClick={() => setConfidence(confidence === c ? null : c)}
              className={cn("rounded border px-1.5", confidence === c ? "border-accent text-accent" : "border-line")}
            >
              {c}
            </button>
          ))}
          <span className="text-subtle">(1 = guessing, 5 = certain)</span>
        </div>
      )}
      {result ? (
        <div className="flex flex-col gap-1">
          <p
            className={cn(
              "flex items-center gap-1.5 text-sm font-medium",
              result.correct ? "text-accent-strong" : "text-danger",
            )}
          >
            {result.correct ? (
              <CircleCheck className="size-4" aria-hidden />
            ) : (
              <CircleX className="size-4" aria-hidden />
            )}
            {result.correct ? "Correct" : "Not quite"}
            {question.kind === "numeric" && ` — answer ${result.answer}${question.unit ?? ""}`}
          </p>
          <Markdown text={result.explanation} />
          {result.computed_by_engine && (
            <p className="text-[11px] text-subtle">This answer is computed by RiverLab&apos;s engine.</p>
          )}
          <div>
            <Button size="sm" variant="ghost" type="button" onClick={retry}>
              Try again
            </Button>
          </div>
        </div>
      ) : (
        <div>
          <Button size="sm" type="submit">
            Check answer
          </Button>
        </div>
      )}
      {error && <ErrorState message={error} />}
    </form>
  );
}

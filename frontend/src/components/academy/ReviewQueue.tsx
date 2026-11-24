"use client";

import { Check, ClipboardCopy, ExternalLink, MessageSquareWarning } from "lucide-react";
import Link from "next/link";
import { useState, useSyncExternalStore } from "react";
import { ApiError, api } from "@/lib/api";
import { type ReviewQueue as Queue, type ReviewQueueItem, type ReviewState, changeRequests } from "@/lib/academy";
import { useGuest } from "@/lib/guest";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Field, TextInput } from "@/components/ui/Field";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";

const NAME_KEY = "riverlab.reviewer";

const STATE_LABEL: Record<ReviewState, string> = {
  to_review: "To review",
  outdated: "Changed since review",
  changes_requested: "Changes requested",
  reviewed: "Reviewed",
};

const STATE_STYLE: Record<ReviewState, string> = {
  to_review: "border-warn/40 text-warn",
  outdated: "border-warn/40 text-warn",
  changes_requested: "border-danger/40 text-danger",
  reviewed: "border-accent/40 text-accent-strong",
};

type Filter = "open" | "changes_requested" | "reviewed" | "all";
const FILTERS: { key: Filter; label: string; match: (s: ReviewState) => boolean }[] = [
  { key: "open", label: "To review", match: (s) => s === "to_review" || s === "outdated" },
  { key: "changes_requested", label: "Changes requested", match: (s) => s === "changes_requested" },
  { key: "reviewed", label: "Reviewed", match: (s) => s === "reviewed" },
  { key: "all", label: "All", match: () => true },
];

function loadName(): string {
  try {
    return window.localStorage.getItem(NAME_KEY) ?? "";
  } catch {
    return "";
  }
}

const noSubscribe = () => () => {};

function saveName(name: string) {
  try {
    window.localStorage.setItem(NAME_KEY, name);
  } catch {
    // not remembered; the field still works
  }
}

/**
 * The lesson review page: a person reads each lesson and marks it reviewed or says what to
 * change. Reviews apply to the lesson's current content only (docs/academy.md, "Review").
 */
export function ReviewQueue() {
  const { data, error, loading, reload } = useApi<Queue>("/api/v1/academy/reviews");
  const guest = useGuest();
  const [filter, setFilter] = useState<Filter>("open");
  // The remembered name, read after hydration; `typed` takes over once the field is edited.
  const stored = useSyncExternalStore(noSubscribe, loadName, () => "");
  const [typed, setName] = useState<string | null>(null);
  const name = typed ?? stored;
  const [copied, setCopied] = useState<string | null>(null);

  if (error) return <ErrorState message={error.message} />;
  if (loading && !data) return <LoadingState />;
  const q = data!;
  const f = FILTERS.find((x) => x.key === filter)!;
  const shown = q.lessons.filter((l) => f.match(l.state));
  const requests = changeRequests(q);
  const open = q.totals.to_review + q.totals.outdated;

  async function copy() {
    try {
      await navigator.clipboard.writeText(requests);
      setCopied("Copied. Paste it into a message asking for the fixes.");
    } catch {
      setCopied(requests); // shown below to copy by hand
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader
          title="How to review a lesson"
          subtitle={`${open} to review · ${q.totals.changes_requested} with changes requested · ${q.totals.reviewed} reviewed`}
        />
        <CardBody className="flex flex-col gap-3 text-sm text-muted">
          <ul className="list-disc space-y-1 pl-5">
            <li>
              Read the lesson and answer its questions. Every number is computed by the engine and checked whenever the
              content syncs, so you don&apos;t need to redo the arithmetic.
            </li>
            <li>Check that each question makes sense on its own, says the street, pot and bet, and has one answer.</li>
            <li>Check the labels: is each claim fairly called a fact, heuristic, approximation or solver result?</li>
            <li>Check that the further-reading works are real and say what the lesson claims they say.</li>
            <li>
              Mark it reviewed only if you are happy putting your name to it. Otherwise request changes and say what to
              fix.
            </li>
          </ul>
          {guest ? (
            <p className="text-warn">Guest mode is on: guests cannot review lessons.</p>
          ) : (
            <div className="max-w-xs">
              <Field label="Your name" hint="Shown on each lesson you review. Remembered on this device.">
                {(id, d) => (
                  <TextInput
                    id={id}
                    aria-describedby={d}
                    value={name}
                    maxLength={100}
                    onChange={(e) => {
                      setName(e.target.value);
                      saveName(e.target.value);
                    }}
                  />
                )}
              </Field>
            </div>
          )}
          {requests && (
            <div className="flex flex-col gap-2">
              <div>
                <Button size="sm" icon={<ClipboardCopy className="size-3.5" aria-hidden />} onClick={copy}>
                  Copy the change requests
                </Button>
              </div>
              {copied &&
                (copied === requests ? (
                  <textarea
                    readOnly
                    aria-label="Change requests"
                    className="h-32 rounded-md border border-line bg-canvas p-2 font-mono text-xs text-fg"
                    value={requests}
                  />
                ) : (
                  <p role="status" className="text-xs text-accent">
                    {copied}
                  </p>
                ))}
            </div>
          )}
        </CardBody>
      </Card>

      <div role="tablist" aria-label="Lessons by review state" className="flex flex-wrap gap-1">
        {FILTERS.map((x) => (
          <Button
            key={x.key}
            role="tab"
            aria-selected={filter === x.key}
            size="sm"
            variant={filter === x.key ? "secondary" : "ghost"}
            onClick={() => setFilter(x.key)}
          >
            {x.label} ({q.lessons.filter((l) => x.match(l.state)).length})
          </Button>
        ))}
      </div>

      {shown.length === 0 ? (
        <EmptyState title="Nothing here" description="No lessons are in this state." />
      ) : (
        shown.map((l) => <LessonReview key={l.slug} lesson={l} name={name} guest={guest} onSaved={reload} />)
      )}
    </div>
  );
}

function LessonReview({
  lesson: l,
  name,
  guest,
  onSaved,
}: {
  lesson: ReviewQueueItem;
  name: string;
  guest: boolean;
  onSaved: () => void;
}) {
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function submit(verdict: "reviewed" | "changes_requested") {
    setErr(null);
    if (!name.trim()) return setErr("Enter your name first.");
    if (verdict === "changes_requested" && !notes.trim()) return setErr("Say what should change.");
    setBusy(true);
    try {
      await api.post(`/api/v1/academy/lessons/${l.slug}/reviews`, { verdict, reviewer: name.trim(), notes });
      setNotes("");
      onSaved();
    } catch (e) {
      setErr(e instanceof ApiError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card aria-label={`Review of ${l.title}`}>
      <CardHeader
        title={l.title}
        subtitle={`Level ${l.level} · v${l.version} · ${l.questions} question${l.questions === 1 ? "" : "s"}${l.status === "in_review" ? " · not published" : ""}`}
        actions={
          <span className={cn("rounded border px-1.5 py-px text-xs", STATE_STYLE[l.state])}>
            {STATE_LABEL[l.state]}
          </span>
        }
      />
      <CardBody className="flex flex-col gap-3 text-sm">
        <Link href={`/academy/${l.slug}`} className="inline-flex items-center gap-1.5 text-accent hover:underline">
          <ExternalLink className="size-3.5" aria-hidden /> Open the lesson
        </Link>
        {l.file_notes && <p className="text-xs text-subtle">Author&apos;s note: {l.file_notes}</p>}
        {l.history.length > 0 && (
          <ul className="flex flex-col gap-1 text-xs text-muted" aria-label="Earlier reviews">
            {l.history.map((h) => (
              <li key={h.id}>
                <span className={h.verdict === "reviewed" ? "text-accent-strong" : "text-danger"}>
                  {h.verdict === "reviewed" ? "Reviewed" : "Changes requested"}
                </span>{" "}
                by {h.reviewer} on {new Date(h.created_at).toLocaleDateString()} (v{h.version}
                {h.current ? "" : ", an older version"}){h.notes && `: ${h.notes}`}
              </li>
            ))}
          </ul>
        )}
        {!guest && (
          <>
            <label className="flex flex-col gap-1">
              <span className="label-caps">Notes</span>
              <textarea
                value={notes}
                maxLength={5000}
                onChange={(e) => setNotes(e.target.value)}
                placeholder="What should change? (optional when marking it reviewed)"
                className="h-20 rounded-md border border-line bg-canvas p-2 text-sm text-fg placeholder:text-subtle focus:border-accent focus:outline-none"
              />
            </label>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="primary"
                loading={busy}
                icon={<Check className="size-3.5" aria-hidden />}
                onClick={() => submit("reviewed")}
              >
                Mark reviewed
              </Button>
              <Button
                size="sm"
                loading={busy}
                icon={<MessageSquareWarning className="size-3.5" aria-hidden />}
                onClick={() => submit("changes_requested")}
              >
                Request changes
              </Button>
            </div>
            {err && (
              <p role="alert" className="text-xs text-danger">
                {err}
              </p>
            )}
          </>
        )}
      </CardBody>
    </Card>
  );
}

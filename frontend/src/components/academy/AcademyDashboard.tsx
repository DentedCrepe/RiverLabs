"use client";

import { ArrowRight, BookOpen, ClipboardCheck, Flame, RotateCcw, Trash2 } from "lucide-react";
import Link from "next/link";
import { api } from "@/lib/api";
import { type CurriculumLevel, type Dashboard, conceptName } from "@/lib/academy";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { EmptyState, ErrorState, LoadingState } from "@/components/ui/States";
import { Metric } from "@/components/solver/ResultView";
import { cn } from "@/components/ui/cn";

const TOPIC_STYLE: Record<string, string> = {
  completed: "border-accent/50 bg-accent-dim/40 text-accent-strong",
  started: "border-info/50 bg-info-dim/40 text-info",
  available: "border-line-strong text-fg hover:border-accent",
  planned: "border-line text-subtle",
};

export function CurriculumMap({ levels }: { levels: CurriculumLevel[] }) {
  return (
    <ol className="flex flex-col gap-4">
      {levels.map((lv) => (
        <li key={lv.number}>
          <h3 className="text-sm font-semibold text-fg">
            Level {lv.number}: {lv.title}
          </h3>
          <p className="mb-1.5 text-xs text-muted">{lv.summary}</p>
          <ul className="flex flex-wrap gap-1.5">
            {lv.topics.map((t) => (
              <li key={t.slug}>
                {t.lesson ? (
                  <Link
                    href={`/academy/${t.lesson}`}
                    className={cn("inline-block rounded border px-2 py-0.5 text-xs", TOPIC_STYLE[t.status])}
                  >
                    {t.title}
                    {t.status === "completed" && " ✓"}
                  </Link>
                ) : (
                  <span
                    className={cn("inline-block rounded border px-2 py-0.5 text-xs", TOPIC_STYLE.planned)}
                    title="Lesson planned"
                  >
                    {t.title}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ol>
  );
}

export function AcademyDashboard() {
  const dash = useApi<Dashboard>("/api/v1/academy/dashboard");
  const curriculum = useApi<CurriculumLevel[]>("/api/v1/academy/curriculum");
  if (dash.error) return <ErrorState message={dash.error.message} />;
  if (dash.loading && !dash.data) return <LoadingState />;
  const d = dash.data!;
  const accuracy = d.answers.total ? `${Math.round((d.answers.correct / d.answers.total) * 100)}%` : "—";
  return (
    <div className="grid gap-6">
      <div className="grid grid-cols-2 gap-2 lg:grid-cols-5">
        <Metric label="Progress" value={`${d.lessons_completed} / ${d.lessons_published} lessons`} />
        <Metric
          label="Current level"
          value={d.current_level ? `Level ${d.current_level}` : d.lessons_published ? "All done" : "—"}
        />
        <Metric label="Answers" value={`${d.answers.correct} / ${d.answers.total}`} sub={`accuracy ${accuracy}`} />
        <Metric label="Streak" value={`${d.streak_days} day${d.streak_days === 1 ? "" : "s"}`} />
        <Metric
          label="Retention"
          value={d.revision.retention === null ? "—" : `${Math.round(d.revision.retention * 100)}%`}
          sub="Right again when re-asked"
        />
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <Card>
          <CardHeader
            title="Curriculum"
            subtitle="Six levels from foundations to solver theory; planned topics are greyed out"
            icon={<BookOpen className="size-4" />}
          />
          <CardBody>{curriculum.data ? <CurriculumMap levels={curriculum.data} /> : <LoadingState />}</CardBody>
        </Card>
        <div className="flex flex-col gap-6">
          <Card>
            <CardHeader title="Next up" />
            <CardBody className="flex flex-col gap-3">
              {d.recommended ? (
                <Link
                  href={`/academy/${d.recommended.slug}`}
                  className="flex items-center justify-between gap-2 rounded-md border border-accent/40 bg-accent-dim/30 px-3 py-2"
                >
                  <span>
                    <span className="block text-xs text-muted">Recommended · Level {d.recommended.level}</span>
                    <span className="text-sm font-medium text-fg">{d.recommended.title}</span>
                  </span>
                  <ArrowRight className="size-4 text-accent" aria-hidden />
                </Link>
              ) : (
                <EmptyState title="Nothing to recommend" description="You have completed every published lesson." />
              )}
              <Link href="/academy/revision" className="flex items-center gap-2 text-sm text-accent hover:underline">
                <RotateCcw className="size-4" aria-hidden /> Revision: {d.revision.due} question
                {d.revision.due === 1 ? "" : "s"} due
              </Link>
              <Link href="/academy/review" className="flex items-center gap-2 text-sm text-accent hover:underline">
                <ClipboardCheck className="size-4" aria-hidden /> Review lessons
              </Link>
              {d.revision.misconceptions.length > 0 && (
                <div>
                  <p className="label-caps mb-1">Suggested revision topics</p>
                  <ul className="flex flex-wrap gap-1.5 text-xs">
                    {d.revision.misconceptions.map((m) => (
                      <li key={m.concept} className="rounded border border-warn/40 px-1.5 py-0.5 text-warn">
                        {conceptName(m.concept)} ({m.incorrect} wrong)
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              {d.streak_days > 0 && (
                <p className="flex items-center gap-1 text-xs text-muted">
                  <Flame className="size-3.5 text-warn" aria-hidden /> Learning streak: {d.streak_days} day
                  {d.streak_days === 1 ? "" : "s"}
                </p>
              )}
            </CardBody>
          </Card>
          <Card>
            <CardHeader title="Your lessons" />
            <CardBody className="flex flex-col gap-3 text-sm">
              <List
                title="Completed"
                items={d.completed.map((c) => ({ href: `/academy/${c.slug}`, label: c.title }))}
                empty="None yet"
              />
              <List
                title="Saved"
                items={d.saved.map((c) => ({ href: `/academy/${c.slug}`, label: c.title }))}
                empty="Save a lesson to find it here"
              />
              <div>
                <p className="label-caps mb-1">Bookmarked concepts</p>
                {d.bookmarks.length === 0 ? (
                  <p className="text-xs text-subtle">Bookmark a section with the icon next to its title</p>
                ) : (
                  <ul className="flex flex-col gap-1">
                    {d.bookmarks.map((b) => (
                      <li key={b.id} className="flex items-center gap-2">
                        <Link href={`/academy/${b.lesson}#${b.section}`} className="text-accent hover:underline">
                          {b.label}
                        </Link>
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label="Remove bookmark"
                          icon={<Trash2 className="size-3" />}
                          onClick={async () => {
                            await api.delete(`/api/v1/academy/bookmarks/${b.id}`);
                            dash.reload();
                          }}
                        />
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div>
                <p className="label-caps mb-1">Knowledge checks</p>
                {d.assessments.length === 0 ? (
                  <p className="text-xs text-subtle">Answer a lesson&apos;s questions to see results here</p>
                ) : (
                  <ul className="flex flex-col gap-1 text-xs">
                    {d.assessments.map((a) => (
                      <li key={a.lesson} className="flex justify-between gap-2">
                        <Link href={`/academy/${a.lesson}`} className="text-accent hover:underline">
                          {a.title}
                        </Link>
                        <span className="font-mono text-muted tabular">
                          {a.correct}/{a.answered} correct · {a.questions - a.answered} unanswered
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </CardBody>
          </Card>
        </div>
      </div>
    </div>
  );
}

function List({ title, items, empty }: { title: string; items: { href: string; label: string }[]; empty: string }) {
  return (
    <div>
      <p className="label-caps mb-1">{title}</p>
      {items.length === 0 ? (
        <p className="text-xs text-subtle">{empty}</p>
      ) : (
        <ul className="flex flex-col gap-1">
          {items.map((i) => (
            <li key={i.href}>
              <Link href={i.href} className="text-accent hover:underline">
                {i.label}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

"use client";

import { Bookmark, BookmarkCheck, CheckCircle2, Clock, ExternalLink, Star } from "lucide-react";
import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import {
  type Block,
  type CurriculumLevel,
  type ExplanationLevel,
  LEVELS,
  LEVEL_LABEL,
  type Lesson,
  blocksForLevel,
  otherLevels,
} from "@/lib/academy";
import { useApi } from "@/lib/useApi";
import { Button } from "@/components/ui/Button";
import { ErrorState, LoadingState } from "@/components/ui/States";
import { cn } from "@/components/ui/cn";
import { Markdown } from "./Markdown";
import { QuestionCard } from "./QuestionCard";
import { TutorPanel } from "./TutorPanel";
import { LessonComponent, LessonQuestions } from "./registry";

const CALLOUT: Record<string, { label: string; className: string }> = {
  fact: { label: "Established fact", className: "border-accent/40 bg-accent-dim/30" },
  heuristic: { label: "Heuristic (rule of thumb)", className: "border-info/40 bg-info-dim/30" },
  approximation: { label: "Approximation: know the limits", className: "border-warn/40 bg-warn-dim/30" },
  solver: { label: "Solver-specific result", className: "border-line-strong bg-surface-2" },
};

/** The same section explained at the other levels, collapsed, so nothing is out of reach. */
function OtherLevels({ groups }: { groups: { level: (typeof LEVELS)[number]; blocks: Block[] }[] }) {
  if (groups.length === 0) return null;
  return (
    <details className="mt-2 rounded-md border border-line px-3 py-2 text-sm">
      <summary className="cursor-pointer text-xs text-accent">
        Also explained at: {groups.map((g) => LEVEL_LABEL[g.level]).join(", ")}
      </summary>
      {groups.map((g) => (
        <div key={g.level} className="mt-2" aria-label={`${LEVEL_LABEL[g.level]} explanation`}>
          <p className="label-caps mb-1">{LEVEL_LABEL[g.level]}</p>
          {g.blocks.map((b, i) => (
            <BlockView key={i} block={b} />
          ))}
        </div>
      ))}
    </details>
  );
}

function BlockView({ block }: { block: Block }) {
  if (block.type === "markdown") return <Markdown text={block.text} />;
  if (block.type === "component") return <LessonComponent name={block.name} props={block.props} />;
  const c = CALLOUT[block.kind];
  return (
    <aside className={cn("my-3 rounded-md border-l-4 px-3 py-2", c.className)} aria-label={c.label}>
      <p className="mb-1 text-[11px] font-semibold tracking-wide text-muted uppercase">{c.label}</p>
      <Markdown text={block.text} />
    </aside>
  );
}

export function LevelSwitcher({
  level,
  onChange,
}: {
  level: ExplanationLevel;
  onChange: (l: ExplanationLevel) => void;
}) {
  return (
    <div
      role="radiogroup"
      aria-label="Explanation level"
      className="inline-flex rounded-md border border-line p-0.5 text-xs"
    >
      {LEVELS.map((l) => (
        <button
          key={l}
          type="button"
          role="radio"
          aria-checked={level === l}
          onClick={() => onChange(l)}
          className={cn("rounded px-2 py-1", level === l ? "bg-accent text-canvas" : "text-muted hover:text-fg")}
        >
          {LEVEL_LABEL[l]}
        </button>
      ))}
    </div>
  );
}

export function LessonView({ slug }: { slug: string }) {
  const { data: lesson, error, loading, reload } = useApi<Lesson>(`/api/v1/academy/lessons/${slug}`);
  const curriculum = useApi<CurriculumLevel[]>("/api/v1/academy/curriculum");
  const [level, setLevel] = useState<ExplanationLevel | null>(null);
  const current = useRef<string | null>(null);
  const sectionRefs = useRef(new Map<string, HTMLElement>());
  const lvl = level ?? lesson?.explanation_level ?? "simple";

  const save = useCallback(
    (changes: Record<string, unknown>) =>
      api.post(`/api/v1/academy/lessons/${slug}/progress`, changes).catch(() => undefined),
    [slug],
  );

  // Track the section being read (the last one whose top has scrolled past 30% of the viewport).
  useEffect(() => {
    if (!lesson) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const onScroll = () => {
      let id: string | null = null;
      for (const [sid, el] of sectionRefs.current) {
        if (el.getBoundingClientRect().top < window.innerHeight * 0.3) id = sid;
      }
      if (id && id !== current.current) {
        current.current = id;
        clearTimeout(timer);
        timer = setTimeout(() => save({ last_section: id }), 1000);
      }
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      window.removeEventListener("scroll", onScroll);
      clearTimeout(timer);
    };
  }, [lesson, save]);

  function changeLevel(l: ExplanationLevel) {
    const keep = current.current;
    setLevel(l);
    save({ explanation_level: l });
    // Keep the reader on the same section after the text above it changes length.
    if (keep) requestAnimationFrame(() => sectionRefs.current.get(keep)?.scrollIntoView({ block: "start" }));
  }

  if (error) return <ErrorState message={error.message} />;
  if (loading || !lesson) return <LoadingState label="Loading lesson…" />;
  const topicLesson = new Map(curriculum.data?.flatMap((lv) => lv.topics.map((t) => [t.slug, t] as const)) ?? []);
  const completed = lesson.progress?.status === "completed";
  const bookmarked = new Set(lesson.bookmarks.map((b) => b.section));

  return (
    <LessonQuestions.Provider value={lesson.questions}>
      <article className="flex flex-col gap-5">
        <header className="flex flex-col gap-3">
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted">
            <Link href="/academy" className="text-accent hover:underline">
              Academy
            </Link>
            <span>· Level {lesson.level}</span>
            {lesson.estimated_minutes && (
              <span className="inline-flex items-center gap-1">
                <Clock className="size-3" aria-hidden /> {lesson.estimated_minutes} min
              </span>
            )}
            <span>· v{lesson.version}</span>
            {lesson.review.status === "pending" ? (
              <span className="rounded border border-warn/40 px-1.5 text-warn" title={lesson.review.notes}>
                Not yet reviewed by a person
              </span>
            ) : (
              <span className="rounded border border-accent/40 px-1.5 text-accent-strong">
                Reviewed by {lesson.review.reviewer} ({lesson.review.reviewed_on})
              </span>
            )}
            {lesson.status !== "published" && (
              <span className="rounded bg-surface-3 px-1.5 uppercase">{lesson.status}</span>
            )}
          </div>
          <h1 className="text-2xl font-semibold text-fg">{lesson.title}</h1>
          <p className="text-sm text-muted">{lesson.summary}</p>
          <div className="grid gap-3 md:grid-cols-2">
            <section aria-label="Learning objectives">
              <h2 className="label-caps mb-1">You will learn to</h2>
              <ul className="list-disc space-y-0.5 pl-5 text-sm text-fg">
                {lesson.meta.objectives.map((o) => (
                  <li key={o}>{o}</li>
                ))}
              </ul>
            </section>
            {lesson.meta.prerequisites.length > 0 && (
              <section aria-label="Prerequisites">
                <h2 className="label-caps mb-1">Builds on</h2>
                <ul className="flex flex-wrap gap-1.5 text-xs">
                  {lesson.meta.prerequisites.map((p) => {
                    const t = topicLesson.get(p);
                    return (
                      <li key={p}>
                        {t?.lesson ? (
                          <Link
                            href={`/academy/${t.lesson}`}
                            className="rounded border border-line px-1.5 py-0.5 text-accent hover:border-accent"
                          >
                            {t.title}
                          </Link>
                        ) : (
                          <span className="rounded border border-line px-1.5 py-0.5 text-muted" title="Lesson planned">
                            {t?.title ?? p} (planned)
                          </span>
                        )}
                      </li>
                    );
                  })}
                </ul>
              </section>
            )}
          </div>
          <div className="sticky top-0 z-10 -mx-1 flex flex-wrap items-center gap-3 bg-canvas/95 px-1 py-2 backdrop-blur">
            <LevelSwitcher level={lvl} onChange={changeLevel} />
            {lesson.progress?.last_section && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  sectionRefs.current.get(lesson.progress!.last_section!)?.scrollIntoView({ block: "start" })
                }
              >
                Continue where you left off
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              icon={<Star className={cn("size-3.5", lesson.progress?.saved && "fill-current")} />}
              onClick={async () => {
                await save({ saved: !lesson.progress?.saved });
                reload();
              }}
            >
              {lesson.progress?.saved ? "Saved" : "Save lesson"}
            </Button>
          </div>
        </header>

        {lesson.sections.map((s) => (
          <section
            key={s.id}
            id={s.id}
            ref={(el) => {
              if (el) sectionRefs.current.set(s.id, el);
              else sectionRefs.current.delete(s.id);
            }}
            className="scroll-mt-16"
          >
            {s.title && (
              <h2 className="mb-2 flex items-center gap-2 text-lg font-semibold text-fg">
                {s.title}
                <button
                  type="button"
                  aria-label={bookmarked.has(s.id) ? `“${s.title}” is bookmarked` : `Bookmark “${s.title}”`}
                  disabled={bookmarked.has(s.id)}
                  className="text-muted hover:text-accent"
                  onClick={async () => {
                    await api.post("/api/v1/academy/bookmarks", {
                      lesson: slug,
                      section: s.id,
                      label: `${lesson.title}: ${s.title}`,
                    });
                    reload();
                  }}
                >
                  {bookmarked.has(s.id) ? (
                    <BookmarkCheck className="size-4 text-accent" />
                  ) : (
                    <Bookmark className="size-4" />
                  )}
                </button>
              </h2>
            )}
            {blocksForLevel(s, lvl).map((b, i) => (
              <BlockView key={i} block={b} />
            ))}
            <OtherLevels groups={otherLevels(s, lvl)} />
          </section>
        ))}

        <TutorPanel slug={lesson.slug} level={lvl} />

        {lesson.questions.length > 0 && (
          <section aria-label="Knowledge check" className="flex flex-col gap-3">
            <h2 className="text-lg font-semibold text-fg">Knowledge check</h2>
            {lesson.questions.map((q) => (
              <QuestionCard key={q.id} question={q} />
            ))}
          </section>
        )}

        <div className="grid gap-4 md:grid-cols-2">
          {lesson.meta.tools.length > 0 && (
            <section aria-label="RiverLab tools">
              <h2 className="label-caps mb-1">Try it in RiverLab</h2>
              <ul className="flex flex-col gap-1.5 text-sm">
                {lesson.meta.tools.map((t) => (
                  <li key={t.href + t.label}>
                    <Link href={t.href} className="text-accent hover:underline">
                      {t.label}
                    </Link>
                    {t.why && <span className="text-muted"> — {t.why}</span>}
                  </li>
                ))}
              </ul>
            </section>
          )}
          {lesson.meta.reading.length > 0 && (
            <section aria-label="Further reading">
              <h2 className="label-caps mb-1">Further reading</h2>
              <ul className="flex flex-col gap-1.5 text-sm">
                {lesson.meta.reading.map((r) => (
                  <li key={r.title}>
                    <span className="text-fg italic">{r.title}</span>
                    <span className="text-muted">
                      {" "}
                      — {r.authors}, {r.year}
                      {r.note && `. ${r.note}`}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          )}
        </div>
        <div>
          <Button
            variant={completed ? "ghost" : "primary"}
            icon={<CheckCircle2 className="size-4" />}
            onClick={async () => {
              await save({ completed: !completed });
              reload();
            }}
          >
            {completed ? "Completed (undo)" : "Mark lesson complete"}
          </Button>
        </div>
        <p className="flex items-center gap-1 text-[11px] text-subtle">
          <ExternalLink className="size-3" aria-hidden /> Lesson content lives in content/academy/lessons/{slug}.md (see
          docs/academy.md).
        </p>
      </article>
    </LessonQuestions.Provider>
  );
}

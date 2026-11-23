/** RiverLab Academy: API types and the explanation-level rules (docs/academy.md). */

export const LEVELS = ["simple", "intermediate", "advanced", "math"] as const;
export type ExplanationLevel = (typeof LEVELS)[number];
export const LEVEL_LABEL: Record<ExplanationLevel, string> = {
  simple: "Simple",
  intermediate: "Intermediate",
  advanced: "Advanced",
  math: "Mathematical",
};

export type Block =
  | { type: "markdown"; level: ExplanationLevel | null; text: string }
  | { type: "callout"; kind: "fact" | "heuristic" | "approximation" | "solver"; text: string }
  | { type: "component"; name: string; props: Record<string, unknown> };
export type Section = { id: string; title: string; blocks: Block[] };
export type Question = {
  id: string;
  kind: "multiple_choice" | "decision" | "numeric";
  prompt: string;
  options: string[] | null;
  unit: string | null;
  concepts: string[];
};
export type AttemptResult = {
  question_id: string;
  correct: boolean;
  answer: number;
  tolerance: number | null;
  explanation: string;
  computed_by_engine: boolean;
};
export type Review = { status: "pending" | "reviewed"; reviewer?: string; reviewed_on?: string; notes?: string };
export type LessonSummary = {
  slug: string;
  title: string;
  summary: string;
  level: number;
  status: "draft" | "in_review" | "published";
  version: number;
  review: Review;
  estimated_minutes: number | null;
  tags: string[];
  progress: { status: "started" | "completed"; saved: boolean; last_section: string | null } | null;
};
export type Lesson = LessonSummary & {
  explanation_level: ExplanationLevel;
  meta: {
    objectives: string[];
    prerequisites: string[];
    tools: { href: string; label: string; why?: string }[];
    reading: { title: string; authors: string; year: number; note?: string }[];
  };
  sections: Section[];
  questions: Question[];
  bookmarks: Bookmark[];
};
export type Bookmark = { id: string; lesson: string; section: string; label: string; note: string | null };
export type CurriculumLevel = {
  number: number;
  title: string;
  summary: string;
  topics: {
    slug: string;
    title: string;
    lesson: string | null;
    status: "planned" | "available" | "started" | "completed";
  }[];
};
export type Dashboard = {
  lessons_published: number;
  lessons_completed: number;
  current_level: number | null;
  per_level: { level: number; lessons: number; completed: number }[];
  recommended: { slug: string; title: string; level: number } | null;
  completed: { slug: string; title: string; level: number }[];
  assessments: { lesson: string; title: string; questions: number; answered: number; correct: number }[];
  answers: { total: number; correct: number; avg_time_ms: number | null };
  streak_days: number;
  saved: { slug: string; title: string }[];
  bookmarks: Bookmark[];
  revision: { due: number; misconceptions: { concept: string; incorrect: number }[]; retention: number | null };
};
export type Revision = {
  due: { question: Question; last_correct: boolean; due: string }[];
  misconceptions: { concept: string; incorrect: number }[];
  retention: number | null;
};

/**
 * The markdown blocks shown at `level`. Blocks without a level are always shown. Of a section's
 * level-specific blocks, the chosen level is shown; if the section has none for it, the closest
 * lower level is used, else the closest higher one, so every section always has content.
 */
export function blocksForLevel(section: Section, level: ExplanationLevel): Block[] {
  const available = new Set(section.blocks.flatMap((b) => (b.type === "markdown" && b.level ? [b.level] : [])));
  const order = LEVELS.indexOf(level);
  const lower = LEVELS.slice(0, order + 1).reverse();
  const higher = LEVELS.slice(order + 1);
  const shown = [...lower, ...higher].find((l) => available.has(l));
  return section.blocks.filter((b) => b.type !== "markdown" || b.level === null || b.level === shown);
}

/** Readable concept name from a topic slug ("pot-odds" -> "pot odds"). */
export const conceptName = (slug: string) => slug.replaceAll("-", " ");

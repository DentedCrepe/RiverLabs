"use client";

import { createContext, useContext } from "react";
import type { Question } from "@/lib/academy";
import { QuestionCard } from "./QuestionCard";
import { BetMathWidget, CombosWidget, EquityWidget, OutsWidget, PotOddsWidget, RegretMatchingWidget } from "./widgets";

/** Questions of the lesson being shown, for the embedded `question` component. */
export const LessonQuestions = createContext<Question[]>([]);

function EmbeddedQuestion({ id }: { id?: string }) {
  const questions = useContext(LessonQuestions);
  const q = questions.find((x) => x.id.endsWith(`:${id}`));
  return q ? <QuestionCard question={q} /> : <UnknownComponent name={`question ${id}`} />;
}

function UnknownComponent({ name }: { name: string }) {
  return (
    <p className="my-3 rounded border border-warn/40 p-2 text-xs text-warn">
      Interactive component “{name}” is not available.
    </p>
  );
}

/* eslint-disable @typescript-eslint/no-explicit-any -- props come from validated lesson JSON */
/**
 * Lesson components by name. The names must match content/academy/components.json, which the
 * backend uses to validate lessons (a test checks they agree).
 */
export const COMPONENTS: Record<string, (props: any) => React.ReactNode> = {
  "pot-odds": (p) => <PotOddsWidget {...p} />,
  outs: (p) => <OutsWidget {...p} />,
  combos: (p) => <CombosWidget {...p} />,
  "bet-math": (p) => <BetMathWidget {...p} />,
  "regret-matching": (p) => <RegretMatchingWidget {...p} />,
  equity: (p) => <EquityWidget {...p} />,
  question: (p) => <EmbeddedQuestion {...p} />,
};
/* eslint-enable @typescript-eslint/no-explicit-any */

export function LessonComponent({ name, props }: { name: string; props: Record<string, unknown> }) {
  const C = COMPONENTS[name];
  return C ? <>{C(props)}</> : <UnknownComponent name={name} />;
}

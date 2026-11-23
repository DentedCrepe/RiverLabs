"use client";

import { useParams } from "next/navigation";
import { LessonView } from "./LessonView";

export function LessonRoute() {
  const { slug } = useParams<{ slug: string }>();
  return <LessonView key={slug} slug={slug} />;
}

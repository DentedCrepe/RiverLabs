import type { Metadata } from "next";
import { Suspense } from "react";
import { LessonRoute } from "@/components/academy/LessonRoute";
import { LoadingState } from "@/components/ui/States";

export const metadata: Metadata = { title: "Academy lesson" };

export default function LessonPage() {
  // The slug is read on the client (useParams); lessons are fetched from the API at runtime.
  return (
    <Suspense fallback={<LoadingState label="Loading lesson…" />}>
      <LessonRoute />
    </Suspense>
  );
}

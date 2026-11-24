import type { Metadata } from "next";
import { ReviewQueue } from "@/components/academy/ReviewQueue";
import { PageHeader } from "@/components/ui/PageHeader";

export const metadata: Metadata = { title: "Lesson review" };

export default function ReviewPage() {
  return (
    <>
      <PageHeader
        title="Lesson review"
        status="available"
        description="Read each lesson and mark it reviewed, or say what to change. A review covers the lesson's current version; any later change sends it back here."
      />
      <ReviewQueue />
    </>
  );
}

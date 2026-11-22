import Link from "next/link";
import { EmptyState } from "@/components/ui/States";

export default function NotFound() {
  return (
    <EmptyState
      title="Page not found"
      description="This page does not exist."
      action={
        <Link href="/" className="text-sm text-accent hover:text-accent-strong">
          Back to the dashboard
        </Link>
      }
    />
  );
}

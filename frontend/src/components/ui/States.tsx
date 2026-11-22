import { CircleAlert, Inbox, LoaderCircle } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "./cn";

export function LoadingState({ label = "Loading…", className }: { label?: string; className?: string }) {
  return (
    <div role="status" className={cn("flex items-center gap-2 py-6 text-sm text-muted", className)}>
      <LoaderCircle aria-hidden className="size-4 animate-spin" />
      <span>{label}</span>
    </div>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <div aria-hidden className={cn("animate-pulse rounded bg-surface-3", className)} />;
}

export function ErrorState({
  title = "Something went wrong",
  message,
  action,
}: {
  title?: string;
  message: string;
  action?: ReactNode;
}) {
  return (
    <div role="alert" className="flex items-start gap-3 rounded-md border border-danger/30 bg-danger-dim/50 p-3">
      <CircleAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-danger" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-fg">{title}</p>
        <p className="mt-0.5 text-sm break-words text-muted">{message}</p>
        {action && <div className="mt-2">{action}</div>}
      </div>
    </div>
  );
}

export function EmptyState({
  title,
  description,
  icon,
  action,
  className,
}: {
  title: string;
  description: ReactNode;
  icon?: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center rounded-md border border-dashed border-line-strong px-6 py-10 text-center",
        className,
      )}
    >
      <span className="mb-3 text-subtle">{icon ?? <Inbox aria-hidden className="size-6" />}</span>
      <p className="text-sm font-medium text-fg">{title}</p>
      <div className="mt-1 max-w-md text-sm text-muted">{description}</div>
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

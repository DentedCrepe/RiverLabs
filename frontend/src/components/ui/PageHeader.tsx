import type { ReactNode } from "react";
import type { FeatureStatus } from "@/lib/nav";
import { StatusBadge } from "./StatusBadge";

export function PageHeader({
  title,
  description,
  status,
  actions,
}: {
  title: string;
  description: ReactNode;
  status?: FeatureStatus;
  actions?: ReactNode;
}) {
  return (
    <div className="mb-6 flex flex-col gap-3 border-b border-line pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <div className="flex items-center gap-2.5">
          <h1 className="text-xl font-semibold tracking-tight text-fg">{title}</h1>
          {status && <StatusBadge status={status} />}
        </div>
        <p className="mt-1 max-w-3xl text-sm text-muted">{description}</p>
      </div>
      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </div>
  );
}

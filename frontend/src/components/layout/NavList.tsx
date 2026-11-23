"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Suspense } from "react";
import { NAV_ITEMS, isActive } from "@/lib/nav";
import { cn } from "@/components/ui/cn";

const DOT: Record<string, string> = {
  available: "bg-accent",
  // A hollow amber dot: works, but built on an approximate model (labelled on the page).
  approximate: "ring-1 ring-warn",
  partial: "bg-info",
  interface_only: "bg-warn",
  planned: "bg-subtle",
};

/**
 * The current path is request data; reading it inside Suspense lets dynamic routes (e.g.
 * /academy/[slug]) prerender, with no item highlighted until it streams in.
 */
export function NavList({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <Suspense fallback={<NavItems pathname={null} onNavigate={onNavigate} />}>
      <ActiveNavList onNavigate={onNavigate} />
    </Suspense>
  );
}

function ActiveNavList({ onNavigate }: { onNavigate?: () => void }) {
  return <NavItems pathname={usePathname()} onNavigate={onNavigate} />;
}

function NavItems({ pathname, onNavigate }: { pathname: string | null; onNavigate?: () => void }) {
  return (
    <ul className="flex flex-col gap-0.5">
      {NAV_ITEMS.map((item) => {
        const active = pathname !== null && isActive(pathname, item.href);
        const Icon = item.icon;
        return (
          <li key={item.href}>
            <Link
              href={item.href}
              onClick={onNavigate}
              aria-current={active ? "page" : undefined}
              title={item.description}
              className={cn(
                "group flex h-9 items-center gap-3 rounded-md px-2.5 text-sm transition-colors",
                active
                  ? "bg-surface-3 text-fg shadow-[inset_2px_0_0_var(--color-accent)]"
                  : "text-muted hover:bg-surface-2 hover:text-fg",
              )}
            >
              <Icon aria-hidden className={cn("size-4 shrink-0", active ? "text-accent" : "text-subtle")} />
              <span className="flex-1 truncate">{item.label}</span>
              <span
                aria-hidden
                className={cn("size-1.5 rounded-full opacity-70", DOT[item.status])}
                title={item.status}
              />
            </Link>
          </li>
        );
      })}
    </ul>
  );
}

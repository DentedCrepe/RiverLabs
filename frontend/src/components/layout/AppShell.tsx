import type { ReactNode } from "react";
import { GuestBanner } from "./GuestMode";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-dvh">
      <a
        href="#main"
        className="sr-only z-50 rounded-md bg-accent px-3 py-2 text-canvas focus:not-sr-only focus:fixed focus:top-2 focus:left-2"
      >
        Skip to content
      </a>
      <Sidebar />
      <div className="flex min-h-dvh flex-col lg:pl-60">
        <TopBar />
        <GuestBanner />
        <main id="main" className="mx-auto w-full max-w-[1400px] flex-1 px-4 py-6 lg:px-8 lg:py-8">
          {children}
        </main>
        <footer className="border-t border-line px-4 py-3 text-xs text-subtle lg:px-8">
          RiverLab · Values come from the validated engine or the solver with its exploitability shown; unfinished areas
          are labelled as planned.
        </footer>
      </div>
    </div>
  );
}

import type { SolverJob } from "./types";

/** How often to refresh a list of solves: quickly while one is queued or running, slowly otherwise. */
export function jobPollMs(jobs: SolverJob[] | null): number {
  return jobs?.some((j) => j.status === "queued" || j.status === "running") ? 1500 : 15_000;
}

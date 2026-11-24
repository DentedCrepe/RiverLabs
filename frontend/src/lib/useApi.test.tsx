import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { jobPollMs } from "./solverJobs";
import type { SolverJob } from "./types";
import { useApi } from "./useApi";

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function setHidden(hidden: boolean) {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => (hidden ? "hidden" : "visible"),
  });
  document.dispatchEvent(new Event("visibilitychange"));
}

describe("useApi background refresh", () => {
  it("pauses while the page is hidden and refreshes once when it is shown again", async () => {
    vi.useFakeTimers();
    const f = vi.spyOn(globalThis, "fetch").mockImplementation(async () => Response.json({ ok: 1 }));
    renderHook(() => useApi("/x", { refreshMs: 1000 }));
    await act(async () => {});
    expect(f).toHaveBeenCalledTimes(1);
    await act(async () => vi.advanceTimersByTime(1000));
    await act(async () => vi.advanceTimersByTime(1000));
    expect(f).toHaveBeenCalledTimes(3);
    act(() => setHidden(true));
    await act(async () => vi.advanceTimersByTime(5000));
    expect(f).toHaveBeenCalledTimes(3);
    await act(async () => setHidden(false));
    expect(f).toHaveBeenCalledTimes(4);
  });

  it("refreshes solve lists quickly only while a solve is queued or running", () => {
    const job = (status: string) => ({ status }) as SolverJob;
    expect(jobPollMs(null)).toBe(15_000);
    expect(jobPollMs([job("succeeded")])).toBe(15_000);
    expect(jobPollMs([job("succeeded"), job("running")])).toBe(1500);
    expect(jobPollMs([job("queued")])).toBe(1500);
  });
});

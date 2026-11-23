import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { SolverJob } from "@/lib/types";
import { JobList } from "./JobList";
import { SolveForm } from "./SolveForm";
import { StrategyMatrix } from "./StrategyMatrix";

describe("StrategyMatrix", () => {
  it("labels every cell and weights the overall frequencies by range weight", () => {
    render(
      <StrategyMatrix
        actions={["check", "bet 2.2bb"]}
        byClass={[
          { label: "AA", row: 0, col: 0, weight: 6, frequencies: [0, 1] },
          { label: "72o", row: 7, col: 12, weight: 2, frequencies: [1, 0] },
        ]}
      />,
    );
    expect(screen.getAllByTestId("strategy-cell")).toHaveLength(169);
    expect(screen.getByLabelText("AA: check 0%, bet 2.2bb 100%")).toBeInTheDocument();
    expect(screen.getByLabelText("KK: not in range")).toBeInTheDocument();
    // (6 * 1 + 2 * 0) / 8 = 75% bet
    expect(screen.getByLabelText("Overall action frequencies")).toHaveTextContent("bet 2.2bb75.0%");
  });
});

const job = (patch: Partial<SolverJob>): SolverJob => ({
  id: "j1",
  status: "running",
  backend: "local",
  iterations: 120,
  progress: { exploitability_pct_pot: 1.2346 },
  cancel_requested: false,
  error_code: null,
  error_message: null,
  created_at: "2026-11-23T10:00:00Z",
  started_at: null,
  finished_at: null,
  ...patch,
});

describe("JobList", () => {
  it("shows progress, cancels running jobs and surfaces failures", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(Response.json(job({ cancel_requested: true })));
    const onChanged = vi.fn();
    render(
      <JobList
        jobs={[job({}), job({ id: "j2", status: "failed", error_message: "spot too large" })]}
        error={null}
        loading={false}
        selected={null}
        onSelect={() => {}}
        onChanged={onChanged}
      />,
    );
    expect(screen.getAllByText(/1\.235% pot/).length).toBeGreaterThan(0);
    expect(screen.getByText("spot too large")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(fetchSpy).toHaveBeenCalledWith(
      "/api/backend/api/v1/solver/jobs/j1/cancel",
      expect.objectContaining({ method: "POST" }),
    );
    await waitFor(() => expect(onChanged).toHaveBeenCalled());
  });
});

describe("SolveForm", () => {
  it("validates input locally and submits an exact spec", async () => {
    const config = {
      id: "c1",
      name: "Default",
      builtin: true,
      stack_preset: "100bb",
      config: { rake: { percentage_bps: 500, cap: null } },
    };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = String(input);
      if (url.endsWith("/game-configs")) return Response.json([config]);
      return Response.json(job({ status: "queued" }), { status: 202 });
    });
    const onSubmitted = vi.fn();
    render(<SolveForm onSubmitted={onSubmitted} />);
    await screen.findByRole("option", { name: /Default/ });
    fireEvent.change(screen.getByLabelText("Starting pot (bb)"), { target: { value: "6.5005" } });
    await userEvent.click(screen.getByRole("button", { name: /queue solve/i }));
    expect(screen.getByRole("alert")).toHaveTextContent(/starting pot/i);
    expect(onSubmitted).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("Starting pot (bb)"), { target: { value: "6.5" } });
    await userEvent.click(screen.getByRole("button", { name: /queue solve/i }));
    await waitFor(() => expect(onSubmitted).toHaveBeenCalled());
    const post = fetchSpy.mock.calls.find(([u]) => String(u).endsWith("/solver/jobs"));
    const body = JSON.parse(String(post?.[1]?.body));
    expect(body.spec.starting_pot_mbb).toBe(6500);
    expect(body.spec.board).toEqual(["Ks", "7d", "2c"]);
    expect(body.spec.game).toEqual(config.config);
  });
});

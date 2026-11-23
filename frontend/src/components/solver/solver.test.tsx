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
  kind: "postflop",
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

describe("NodeBrowser", () => {
  it("steps through decisions and dealt cards and back via the breadcrumb", async () => {
    const { NodeBrowser } = await import("./NodeBrowser");
    const decision = {
      kind: "decision",
      street: "turn",
      board: ["Ks", "7d", "2c", "8h"],
      put_mbb: [3250, 3250],
      player: 0,
      actions: ["check", "bet 2.2bb"],
      cards: [],
      by_class: [{ label: "AA", row: 0, col: 0, weight: 6, frequencies: [0.25, 0.75] }],
      reach_fraction: [1, 1],
      path: "",
    };
    const chance = {
      ...decision,
      kind: "chance",
      player: null,
      actions: [],
      by_class: [],
      cards: ["Ah", "2s"],
      path: "a0.a0",
    };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const path = new URL(String(input), "http://x").searchParams.get("path") ?? "";
      return Response.json(path === "a0.a0" ? chance : { ...decision, path, player: path === "a0" ? 1 : 0 });
    });
    render(<NodeBrowser jobId="j1" />);
    await userEvent.click(await screen.findByRole("button", { name: /check 25\.0%/ }));
    expect(await screen.findByText(/IP to act/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /check 25\.0%/ }));
    expect(await screen.findByText(/Deal the river/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /ace of hearts/i }));
    await waitFor(() =>
      expect(fetchSpy).toHaveBeenLastCalledWith(expect.stringContaining("path=a0.a0.cAh"), expect.anything()),
    );
    await userEvent.click(screen.getByRole("button", { name: "Root" }));
    expect(await screen.findByText(/OOP to act/)).toBeInTheDocument();
  });
});

describe("NodeBrowser following a real line", () => {
  it("walks to your decision and shows the solver mix for your hand", async () => {
    const { NodeBrowser } = await import("./NodeBrowser");
    const base = {
      kind: "decision",
      street: "flop",
      board: ["Ks", "7d", "2c"],
      put_mbb: [2500, 2500],
      cards: [],
      reach_fraction: [1, 1],
    };
    const nodes: Record<string, object> = {
      "": { ...base, player: 0, actions: ["check", "bet 1.6bb", "bet 3.8bb"], by_class: [] },
      // The bot bet 3.75bb (75%): the nearest bet in the tree is 3.8bb.
      a2: {
        ...base,
        player: 1,
        actions: ["fold", "call 3.8bb", "raise to 12.5bb"],
        by_class: [{ label: "AKo", row: 0, col: 1, weight: 12, frequencies: [0.1, 0.6, 0.3] }],
      },
    };
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const path = new URL(String(input), "http://x").searchParams.get("path") ?? "";
      return Response.json({ ...nodes[path], path });
    });
    render(
      <NodeBrowser
        jobId="j2"
        follow={{
          line: [{ player: 0, kind: "bet", to_mbb: 3750, all_in: false }],
          heroPlayer: 1,
          heroAction: { player: 1, kind: "call", to_mbb: 3750, all_in: false },
          heroClass: "AKo",
          heroCards: ["As", "Kd"],
        }}
      />,
    );
    expect(await screen.findByText(/Your decision in the hand/)).toBeInTheDocument();
    expect(screen.getByText(/solver mix for AKo/)).toBeInTheDocument();
    expect(screen.getByText("call 3.8bb ← you").parentElement).toHaveTextContent("60.0%");
    expect(screen.getByRole("button", { name: "bet 3.8bb" })).toBeInTheDocument();
  });
});

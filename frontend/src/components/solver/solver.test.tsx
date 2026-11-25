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
        jobs={[job({ attempts: 2 }), job({ id: "j2", status: "failed", error_message: "spot too large" })]}
        error={null}
        loading={false}
        selected={null}
        onSelect={() => {}}
        onChanged={onChanged}
      />,
    );
    expect(screen.getAllByText(/1\.235% pot/).length).toBeGreaterThan(0);
    expect(screen.getByText("spot too large")).toBeInTheDocument();
    expect(screen.getByText("restarted (attempt 2)")).toBeInTheDocument();
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

describe("node locking", () => {
  // Shapes only: the frequencies are test inputs, not solver output.
  const decision = {
    kind: "decision",
    street: "river",
    board: ["Ks", "Qd", "7h", "7c", "2s"],
    put_mbb: [5000, 5000],
    player: 1,
    actions: ["check", "bet 10bb"],
    cards: [],
    by_class: [{ label: "KK", row: 1, col: 1, weight: 4, frequencies: [0.75, 0.25] }],
    reach_fraction: [1, 1],
    path: "a0",
  };

  it("locks a decision from the tree and re-solves with the lock added to the spec", async () => {
    const { ResultView } = await import("./ResultView");
    const posts: unknown[] = [];
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      const u = String(input);
      if (init?.method === "POST") {
        posts.push(JSON.parse(String(init.body)));
        return Response.json({ id: "j2", status: "queued" }, { status: 202 });
      }
      if (u.includes("/result"))
        return Response.json({
          job_id: "j1",
          engine_version: "0",
          solver_backend: "gtopen",
          solver_version: "x",
          algorithm: "dcfr",
          iterations: 10,
          stop_reason: "max_iterations",
          exploitability_mbb: 1,
          exploitability_pct_pot: 0.01,
          values_mbb: [0, 0],
          elapsed_ms: 1,
          memory_bytes: 1,
          approximations: [],
          independent_check: { performed: false, values_mbb: null, exploitability_mbb: null, note: "" },
          root: null,
          tree_available: true,
          locks: [],
          unlocked: null,
          spec: { board: ["Ks"], solver: {} },
        });
      const path = new URL(u, "http://x").searchParams.get("path") ?? "";
      return Response.json({ ...decision, path, player: path ? 1 : 0, actions: path ? decision.actions : ["check"] });
    });
    const onLocked = vi.fn();
    const user = userEvent.setup();
    render(<ResultView jobId="j1" onLocked={onLocked} />);
    await user.click(await screen.findByRole("button", { name: /^check/ }));
    await screen.findByText(/IP to act/);
    await user.click(screen.getByText("Lock this decision"));
    const bet = screen.getByLabelText("bet 10bb (%)");
    expect(screen.getByLabelText("check (%)")).toHaveValue(75);
    fireEvent.change(bet, { target: { value: "50" } });
    await user.click(screen.getByRole("button", { name: "Solve with this lock" }));
    expect(await screen.findByText(/add up to 125%/)).toHaveAttribute("role", "alert");
    fireEvent.change(screen.getByLabelText("check (%)"), { target: { value: "50" } });
    await user.click(screen.getByRole("button", { name: "Solve with this lock" }));
    await waitFor(() => expect(onLocked).toHaveBeenCalledWith({ id: "j2", status: "queued" }));
    expect(posts).toEqual([{ spec: { board: ["Ks"], solver: {}, locks: [{ path: [0], frequencies: [0.5, 0.5] }] } }]);
  });

  it("labels a locked result as an exploit and shows how the values moved", async () => {
    const { ResultView } = await import("./ResultView");
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      if (String(input).includes("/result"))
        return Response.json({
          job_id: "j2",
          engine_version: "0",
          solver_backend: "gtopen",
          solver_version: "x",
          algorithm: "dcfr",
          iterations: 10,
          stop_reason: "max_iterations",
          exploitability_mbb: 1,
          exploitability_pct_pot: 0.01,
          values_mbb: [1500, -1500],
          elapsed_ms: 1,
          memory_bytes: 1,
          approximations: [],
          independent_check: { performed: false, values_mbb: null, exploitability_mbb: null, note: "" },
          root: null,
          tree_available: true,
          locks: [{ path: [], frequencies: [1, 0] }],
          unlocked: { iterations: 10, exploitability_mbb: 1, exploitability_pct_pot: 0.01, values_mbb: [1000, -1000] },
          spec: {},
        });
      return Response.json({ ...decision, path: "", player: 0 });
    });
    render(<ResultView jobId="j2" />);
    expect(await screen.findByText(/an exploit, not an equilibrium/)).toBeInTheDocument();
    expect(screen.getByText(/OOP \+1bb → \+1\.5bb/)).toBeInTheDocument();
    expect(await screen.findByText(/Locked: OOP plays these frequencies/)).toBeInTheDocument();
    expect(screen.queryByText("Lock this decision")).not.toBeInTheDocument();
  });
});

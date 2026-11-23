import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { HandSolverReview } from "@/lib/play";
import { SolverDecision, SolverReviewPanel } from "./SolverReview";

afterEach(() => vi.restoreAllMocks());

const decision = {
  decision: 2,
  before: ["check"],
  actions: ["check", "bet 3bb"],
  strategy: [0.25, 0.75],
  action_evs_bb: [1.1, 1.4],
  taken: 0,
  best: 1,
  ev_loss_bb: 0.3,
};
const done: HandSolverReview = {
  id: "r1",
  status: "done",
  error_message: null,
  streets: [
    {
      street: "flop",
      decisions: [1],
      solvable: false,
      note: "Not solvable: more than two players.",
      status: "skipped",
    },
    {
      street: "turn",
      decisions: [2],
      solvable: true,
      note: null,
      status: "done",
      job_id: "j1",
      notes: ["Tree: on the turn, bets of 75%."],
      review: { decisions: [decision], followed: true, notes: [], next_ranges: null },
    },
  ],
};

describe("solver review", () => {
  it("offers to start, then summarises each street and the EV you lost", async () => {
    const onStart = vi.fn();
    const { rerender } = render(<SolverReviewPanel review={null} error={null} starting={false} onStart={onStart} />);
    await userEvent.click(screen.getByRole("button", { name: /Review this hand with the solver/ }));
    expect(onStart).toHaveBeenCalled();
    rerender(<SolverReviewPanel review={done} error={null} starting={false} onStart={onStart} />);
    expect(screen.getByText(/1 decision · EV lost 0.30bb/)).toBeInTheDocument();
    const streets = screen.getByLabelText("Streets");
    expect(within(streets).getByText("not solvable")).toBeInTheDocument();
    expect(within(streets).getByText("solved")).toBeInTheDocument();
    expect(screen.getByText(/What these solves assume/)).toBeInTheDocument();
  });

  it("shows the mix for your hand, each action's EV, your choice and the best one", () => {
    render(<SolverDecision r={decision} />);
    const view = screen.getByLabelText("Solver view of this decision");
    expect(view).toHaveTextContent("EV lost 0.30bb");
    expect(screen.getByText("check ← you")).toBeInTheDocument();
    expect(screen.getByText("bet 3bb ★")).toBeInTheDocument();
    expect(view).toHaveTextContent("75%");
    expect(view).toHaveTextContent("+1.40bb");
  });
});

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { addToTally, EMPTY_TALLY, type Grade, signedBb } from "@/lib/trainer";
import { PushFoldTrainer } from "./PushFoldTrainer";

const drill = { stack_mbb: 10_000, seat: "sb", cards: ["Ks", "7d"], class: "K7o" };
const grade: Grade = {
  drill: drill as Grade["drill"],
  choice: "fold",
  actions: ["fold", "jam"],
  frequencies: [0, 1],
  evs_bb: [-0.5, 0.412],
  ev_loss_bb: 0.912,
  verdict: "mistake",
  exploitability_bb: 1e-6,
};

afterEach(() => vi.restoreAllMocks());

describe("push/fold trainer", () => {
  it("offers to build the table when it is missing", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      Response.json({ state: "missing", progress: null, message: null, table_present: false }),
    );
    render(<PushFoldTrainer />);
    expect(await screen.findByRole("button", { name: /Build the table here/ })).toBeInTheDocument();
  });

  it("deals, grades by EV and keeps a tally", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = String(input);
      if (url.endsWith("/drill")) return Response.json(drill);
      if (url.endsWith("/answer")) return Response.json(grade);
      return Response.json({ state: "ready", progress: null, message: null, table_present: true });
    });
    render(<PushFoldTrainer />);
    await userEvent.click(await screen.findByRole("button", { name: "Deal" }));
    expect(await screen.findByText(/10bb effective · Small blind/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Fold" }));
    expect(await screen.findByText("Mistake")).toBeInTheDocument();
    expect(screen.getByText(/You lost 0.912bb/)).toBeInTheDocument();
    expect(screen.getByText("+0.412bb")).toBeInTheDocument();
    expect(screen.getByLabelText("Session tally")).toHaveTextContent("Mistakes1");
    const answer = fetchSpy.mock.calls.find(([u]) => String(u).endsWith("/answer"));
    expect(JSON.parse(String(answer?.[1]?.body))).toEqual({ drill, choice: "fold" });
  });

  it("formats signed bb and adds up the tally", () => {
    expect(signedBb(-0.5)).toBe("−0.5bb");
    expect(signedBb(0.4123)).toBe("+0.412bb");
    expect(signedBb(-0.0001)).toBe("+0bb");
    const t = addToTally(addToTally(EMPTY_TALLY, grade), { ...grade, verdict: "correct", ev_loss_bb: 0 });
    expect(t).toEqual({ hands: 2, correct: 1, close: 0, mistakes: 1, evLost: 0.912 });
  });
});

describe("ICM push/fold trainer", () => {
  it("deals a spot, grades it in prize-pool share and labels the model", async () => {
    const { IcmTrainer } = await import("./IcmTrainer");
    // Shape-only fixture: not engine output.
    const icmDrill = {
      spot: { stacks: [8000, 12000, 5000, 9000], small_blind: 500, big_blind: 1000, ante: 0, payouts: [50, 30, 20] },
      positions: ["CO", "BTN", "SB", "BB"],
      hero: 3,
      facing: 1,
      cards: ["Ah", "Td"],
      class: "ATo",
    };
    const icmGrade = {
      drill: icmDrill,
      choice: "play",
      actions: ["fold", "call"],
      frequencies: [0.2, 0.8],
      values_pct: [24.1, 25.3],
      loss_pct: 0,
      verdict: "correct",
      epsilon_pct: 0.0002,
      iterations: 150,
      approximations: ["ICM (Malmuth–Harville) values each outcome."],
    };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = String(input);
      if (url.endsWith("/icm/drill")) return Response.json(icmDrill);
      if (url.endsWith("/icm/answer")) return Response.json(icmGrade);
      return Response.json({ state: "ready", progress: null, message: null, table_present: true });
    });
    render(<IcmTrainer />);
    await userEvent.click(await screen.findByRole("button", { name: "Deal" }));
    const stacks = await screen.findByLabelText("Stacks in order of action");
    expect(stacks).toHaveTextContent("BTN 12bb · all-in");
    expect(stacks).toHaveTextContent("SB 5bb · folded");
    expect(stacks).toHaveTextContent("BB 9bb (you)");
    const body = JSON.parse(
      String((fetchSpy.mock.calls.find((c) => String(c[0]).endsWith("/icm/drill"))![1] as RequestInit).body),
    );
    expect(body.payouts).toEqual([50, 30, 20]);
    await userEvent.click(screen.getByRole("button", { name: "Call" }));
    const g = await screen.findByLabelText("ICM grade");
    expect(g).toHaveTextContent("Correct");
    expect(g).toHaveTextContent("25.30%");
    expect(g).toHaveTextContent("modelled approximation");
  });
});

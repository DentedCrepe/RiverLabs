import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PreflopTrainer, type PreflopDrill, preflopTable } from "./PreflopTrainer";

// Shapes only: the frequencies below are test inputs, not model output.
const label = {
  model: "GTOpen preflop model (approximate; flop pots are modelled, not solved)",
  validated_scope: "approximate_model",
  model_notes: ["Approximate model."],
  model_gap_mbb: 12,
  iterations: 30,
};

const drill: PreflopDrill = {
  spot: "facing_raise",
  path: [1],
  position: "SB",
  class: "AKo",
  cards: ["As", "Kd"],
  positions: ["BTN", "SB", "BB"],
  pot_mbb: 4000,
  invested_mbb: [2500, 500, 1000],
  live: [true, true, true],
  actions: [
    { label: "fold", kind: "fold", to_mbb: 500 },
    { label: "call", kind: "call", to_mbb: 2500 },
    { label: "raise 7.5", kind: "raise", to_mbb: 7500 },
  ],
  history: [{ actor: "BTN", actions: ["fold", "raise 2.5"], chosen: 1 }],
};

describe("preflop trainer", () => {
  it("draws the table with you first and the raise in front of the button", () => {
    const t = preflopTable(drill, 100_000);
    expect(t.seats.map((s) => s.position)).toEqual(["SB", "BB", "BTN"]);
    expect(t.seats[0]).toMatchObject({ is_hero: true, cards: ["As", "Kd"], stack: 99_500 });
    expect(t.seats[2]).toMatchObject({ is_button: true, last_action: "raise 2.5", street_commit: 2500 });
  });

  it("deals from a saved preflop solve, labels the model and grades by frequency", async () => {
    const posts: { url: string; body: unknown }[] = [];
    vi.spyOn(globalThis, "fetch").mockImplementation(async (url, init) => {
      const u = String(url);
      if (init?.method === "POST") posts.push({ url: u, body: JSON.parse(String(init.body)) });
      if (u.endsWith("/trainer/preflop/sources"))
        return Response.json([
          { ...label, job_id: "j1", positions: ["BTN", "SB", "BB"], effective_stack_mbb: 100_000, rake_bps: 500 },
        ]);
      if (u.endsWith("/trainer/preflop/drill"))
        return Response.json({ ...label, job_id: "j1", drill, effective_stack_mbb: 100_000 });
      return Response.json({
        ...label,
        drill,
        choice: 1,
        choice_label: "call",
        verdict: "mixed",
        your_frequency: 0.3,
        model_action: 2,
        actions: [
          { label: "fold", kind: "fold", frequency: 0.1 },
          { label: "call", kind: "call", frequency: 0.3 },
          { label: "raise 7.5", kind: "raise", frequency: 0.6 },
        ],
        by_class: [],
        mixed_min: 0.15,
      });
    });
    const user = userEvent.setup();
    render(<PreflopTrainer />);
    expect(await screen.findByRole("note")).toHaveTextContent("Approximate model, not a solved game");
    await user.selectOptions(screen.getByLabelText("Spot"), "facing_raise");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    expect(await screen.findByText(/BTN raise 2.5/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "call" }));
    expect(await screen.findByText("Mixed")).toBeInTheDocument();
    expect(
      screen.getByText(/plays call with AKo 30% of the time; its most frequent play is raise 7.5 \(60%\)/),
    ).toBeInTheDocument();
    expect(screen.getByText(/not by EV/)).toBeInTheDocument();
    expect(posts[0]).toEqual({
      url: "/api/backend/api/v1/trainer/preflop/drill",
      body: { job_id: null, spot: "facing_raise" },
    });
    expect(posts[1].body).toEqual({ job_id: "j1", drill, choice: 1 });
  });

  it("points to the Preflop page when there is nothing to drill from", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async () => Response.json([]));
    render(<PreflopTrainer />);
    expect(await screen.findByText("No saved preflop solves yet")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Go to the Preflop page" })).toHaveAttribute("href", "/preflop");
  });
});

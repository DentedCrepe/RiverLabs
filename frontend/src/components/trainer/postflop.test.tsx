import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type PostflopDrill, PostflopTrainer, drillTable } from "./PostflopTrainer";

afterEach(() => vi.restoreAllMocks());

const drill: PostflopDrill = {
  hero_player: 1,
  hero_combo: "KhKd",
  hero_class: "KK",
  path: [{ type: "action", value: 0 }],
  history: [{ player: 0, label: "check" }],
  street: "river",
  board: ["Ks", "Qd", "7h", "7c", "2s"],
  pot_mbb: 10000,
  street_bets_mbb: [0, 0],
  stacks_mbb: [10000, 10000],
  actions: ["check", "bet 10bb"],
};

describe("postflop trainer", () => {
  it("asks for a saved solve first", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(Response.json([]));
    render(<PostflopTrainer />);
    expect(await screen.findByText("No saved solves yet")).toBeInTheDocument();
  });

  it("deals a spot on the table, then grades the answer with the solver's mix and EVs", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = String(input);
      if (url.endsWith("/sources"))
        return Response.json([
          {
            job_id: "j1",
            board: drill.board,
            starting_pot_mbb: 10000,
            effective_stack_mbb: 10000,
            exploitability_pct_pot: 0.2,
          },
        ]);
      if (url.endsWith("/drill"))
        return Response.json({ job_id: "j1", drill, starting_pot_mbb: 10000, exploitability_pct_pot: 0.2 });
      return Response.json({
        actions: drill.actions,
        strategy: [0, 1],
        action_evs_bb: [4.5, 7.25],
        taken: 0,
        best: 1,
        ev_loss_bb: 2.75,
        ev_loss_pct_pot: 27.5,
        verdict: "mistake",
      });
    });
    render(<PostflopTrainer />);
    await userEvent.click(await screen.findByRole("button", { name: "Deal" }));
    const spot = await screen.findByLabelText("Current postflop drill");
    expect(within(spot).getByText(/You are in position/)).toBeInTheDocument();
    expect(within(spot).getByText(/OOP check/)).toBeInTheDocument();
    await userEvent.click(within(spot).getByRole("button", { name: "check" }));
    expect(await screen.findByText("Mistake")).toBeInTheDocument();
    expect(screen.getByText(/You lost 2.75bb \(27.5% of the starting pot\)/)).toBeInTheDocument();
    expect(screen.getByText("bet 10bb ★")).toBeInTheDocument();
    expect(screen.getByLabelText("Postflop tally")).toHaveTextContent("1 answered · 0% correct");
    const answer = fetchSpy.mock.calls.find(([u]) => String(u).endsWith("/answer"));
    expect(JSON.parse(String(answer?.[1]?.body))).toEqual({ job_id: "j1", drill, choice: 0 });
  });

  it("draws you at the bottom, with the in-position player on the button", () => {
    const t = drillTable(drill);
    expect(t.seats[0]).toMatchObject({ name: "You", position: "IP", is_button: true, cards: ["Kh", "Kd"] });
    expect(t.seats[1]).toMatchObject({ name: "Opponent", position: "OOP", last_action: "check", cards: null });
    expect(t.pot).toBe(10000);
  });
});

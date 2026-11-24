import userEvent from "@testing-library/user-event";
import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Analytics } from "./Analytics";

const empty = { answers: 0, correct: 0, close: 0, mistakes: 0, ev_lost_bb: 0 };

const emptyPostflop = {
  total: empty,
  by_street: [],
  by_position: { oop: empty, ip: empty },
  costliest: [],
};

const zero = { queued: 0, running: 0, done: 0, failed: 0, interrupted: 0 };
const noLeaks = {
  reviews: 0,
  threshold_pct_pot: 1,
  total: { decisions: 0, leaks: 0, ev_lost_bb: 0 },
  by_street: [],
  by_position: [],
  by_category: [],
  costliest: [],
};

const noPreflop = { answers: 0, matches: 0, mixed: 0, differs: 0 };
const emptyPreflop = { total: noPreflop, by_spot: [], by_position: [], most_differing: [] };

/** Answers for the endpoints every Analytics section loads besides the one under test. */
function common(url: string): Response | null {
  if (url.includes("/reviews/status")) return Response.json({ reviews: zero });
  if (url.includes("/analytics/leaks")) return Response.json(noLeaks);
  if (url.includes("/analytics/preflop")) return Response.json(emptyPreflop);
  return null;
}

afterEach(() => vi.restoreAllMocks());

describe("Analytics", () => {
  it("shows empty states instead of sample data", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(
      async (input) =>
        common(String(input)) ??
        (String(input).includes("/analytics/postflop")
          ? Response.json(emptyPostflop)
          : String(input).includes("/analytics/play")
            ? Response.json({ scope: "cash", sessions: [], report: null })
            : Response.json({
                total: empty,
                by_seat: { sb: empty, bb: empty },
                by_stack: [],
                costliest: [],
                recent: [],
              })),
    );
    render(<Analytics />);
    expect(await screen.findByText("No hands yet")).toBeInTheDocument();
    expect((await screen.findAllByText("No answers yet")).length).toBe(3);
  });

  it("summarises trainer answers by seat, stack and costliest hand", async () => {
    const sb = { answers: 3, correct: 1, close: 0, mistakes: 2, ev_lost_bb: 0.6 };
    vi.spyOn(globalThis, "fetch").mockImplementation(
      async (input) =>
        common(String(input)) ??
        (String(input).includes("/analytics/postflop")
          ? Response.json(emptyPostflop)
          : String(input).includes("/analytics/play")
            ? Response.json({ scope: "cash", sessions: [], report: null })
            : Response.json({
                total: sb,
                by_seat: { sb, bb: empty },
                by_stack: [{ ...sb, min_mbb: 2000, max_mbb: 5500 }],
                costliest: [{ ...sb, hand_class: "K7o", seat: "sb" }],
                recent: [],
              })),
    );
    render(<Analytics />);
    expect((await screen.findAllByText("33%")).length).toBeGreaterThan(0);
    expect(screen.getByText("2–5.5bb")).toBeInTheDocument();
    expect(screen.getByLabelText("Costliest hands")).toHaveTextContent("K7o");
    expect(screen.getByLabelText("Costliest hands")).toHaveTextContent("2 of 3 wrong");
    expect(screen.getAllByText("0.200bb").length).toBeGreaterThan(0);
  });
});

describe("solver leaks", () => {
  it("summarises EV lost by kind of mistake and lets you flick through the costliest decisions", async () => {
    const { LeaksSection } = await import("./Leaks");
    const leaks = {
      ...noLeaks,
      reviews: 2,
      total: { decisions: 5, leaks: 2, ev_lost_bb: 1.5 },
      by_street: [{ street: "turn", decisions: 3, leaks: 1, ev_lost_bb: 1.0 }],
      by_position: [{ position: "BB", decisions: 5, leaks: 2, ev_lost_bb: 1.5 }],
      by_category: [{ category: "Called when folding was better", leaks: 1, ev_lost_bb: 1.0 }],
      costliest: [
        {
          session_id: "s1",
          hand_no: 7,
          street: "turn",
          decision: 3,
          position: "BB",
          taken: "call 5bb",
          best: "fold",
          ev_loss_bb: 1.0,
          ev_loss_pct_pot: 10,
          category: "Called when folding was better",
        },
      ],
    };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      const url = String(input);
      if (url.includes("/reviews/batch")) return Response.json({ queued: [{}, {}], reviews: zero });
      if (url.includes("/reviews/status")) return Response.json({ reviews: zero });
      if (url.includes("/analytics/leaks")) return Response.json(leaks);
      return Response.json({ error: { code: "x", message: String(init?.method) } }, { status: 404 });
    });
    render(<LeaksSection />);
    expect(await screen.findByText("Called when folding was better")).toBeInTheDocument();
    expect(screen.getAllByText("0.300bb").length).toBeGreaterThan(0);
    expect(screen.getByLabelText("Costliest decisions")).toHaveTextContent(
      "Hand 7, turn (BB): call 5bb, best fold · 1.00bb",
    );
    await userEvent.click(screen.getByRole("button", { name: "Review with the solver" }));
    const batch = fetchSpy.mock.calls.find(([u]) => String(u).includes("/reviews/batch"));
    expect(JSON.parse(String(batch?.[1]?.body))).toEqual({ scope: "flagged", limit: 10 });
    await userEvent.click(screen.getByRole("button", { name: /Flick through them \(1\)/ }));
    expect(await screen.findByText(/1 of 1/)).toBeInTheDocument();
  });
});

import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Analytics } from "./Analytics";

const empty = { answers: 0, correct: 0, close: 0, mistakes: 0, ev_lost_bb: 0 };

afterEach(() => vi.restoreAllMocks());

describe("Analytics", () => {
  it("shows empty states instead of sample data", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) =>
      String(input).includes("/analytics/play")
        ? Response.json({ scope: "cash", sessions: [], report: null })
        : Response.json({
            total: empty,
            by_seat: { sb: empty, bb: empty },
            by_stack: [],
            costliest: [],
            recent: [],
          }),
    );
    render(<Analytics />);
    expect(await screen.findByText("No hands yet")).toBeInTheDocument();
    expect(await screen.findByText("No answers yet")).toBeInTheDocument();
  });

  it("summarises trainer answers by seat, stack and costliest hand", async () => {
    const sb = { answers: 3, correct: 1, close: 0, mistakes: 2, ev_lost_bb: 0.6 };
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) =>
      String(input).includes("/analytics/play")
        ? Response.json({ scope: "cash", sessions: [], report: null })
        : Response.json({
            total: sb,
            by_seat: { sb, bb: empty },
            by_stack: [{ ...sb, min_mbb: 2000, max_mbb: 5500 }],
            costliest: [{ ...sb, hand_class: "K7o", seat: "sb" }],
            recent: [],
          }),
    );
    render(<Analytics />);
    expect((await screen.findAllByText("33%")).length).toBeGreaterThan(0);
    expect(screen.getByText("2–5.5bb")).toBeInTheDocument();
    expect(screen.getByLabelText("Costliest hands")).toHaveTextContent("K7o");
    expect(screen.getByLabelText("Costliest hands")).toHaveTextContent("2 of 3 wrong");
    expect(screen.getAllByText("0.200bb").length).toBeGreaterThan(0);
  });
});

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Library, type LibraryEntry } from "./Library";

const base: LibraryEntry = {
  job_id: "j1",
  kind: "postflop",
  finished_at: "2026-11-23T07:01:01Z",
  engine_version: "0.1.0",
  solver_backend: "gtopen",
  solver_version: "eb5ccd671223293527f36d219cfe33baf0b0475c",
  iterations: 100,
  stop_reason: "target_reached",
  tree_available: true,
  board: ["3d", "5c", "Kc"],
  ranges: ["AA", "KK"],
  starting_pot_mbb: 4000,
  effective_stack_mbb: 93000,
  exploitability_mbb: 13.38,
  exploitability_pct_pot: 0.3345,
  independent_check: true,
  approximations: 1,
  players: null,
  positions: null,
  validated_scope: null,
  model_gap_mbb: null,
};
const preflop: LibraryEntry = {
  ...base,
  job_id: "j2",
  kind: "preflop",
  board: null,
  ranges: null,
  starting_pot_mbb: null,
  exploitability_mbb: null,
  exploitability_pct_pot: null,
  independent_check: null,
  players: 6,
  effective_stack_mbb: 100000,
  validated_scope: "approximate_model",
  model_gap_mbb: 4.2,
};

afterEach(() => vi.restoreAllMocks());

describe("Solution Library", () => {
  it("lists results with their quality and labels preflop models as approximate", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = String(input);
      if (url.includes("/library")) return Response.json([base, preflop]);
      return Response.json({ error: { code: "x", message: "not mocked" } }, { status: 404 });
    });
    render(<Library />);
    expect(await screen.findByText("0.335% pot")).toBeInTheDocument();
    expect(screen.getByText(/re-checked/)).toBeInTheDocument();
    expect(screen.getByText(/approximate model/)).toBeInTheDocument();
    expect(screen.getByText(/Preflop · 6 players · 100bb/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "preflop" }));
    expect(fetchSpy).toHaveBeenLastCalledWith(expect.stringContaining("kind=preflop"), expect.anything());
  });

  it("opens a result with its ranges", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) =>
      String(input).includes("/library")
        ? Response.json([base])
        : Response.json({ error: { code: "x", message: "not mocked" } }, { status: 404 }),
    );
    render(<Library />);
    await userEvent.click(await screen.findByRole("button", { expanded: false }));
    expect(screen.getByText("AA")).toBeInTheDocument();
    expect(screen.getByText("KK")).toBeInTheDocument();
  });
});

describe("whole hands in the library", () => {
  it("groups the street solves of one hand into one row and keeps other solves", async () => {
    const { libraryRows } = await import("./Library");
    const street = (id: string, st: string, board: string[]): LibraryEntry => ({
      ...base,
      job_id: id,
      board,
      hand: { session_id: "s1", hand_no: 3, street: st },
    });
    const rows = libraryRows([
      street("r", "river", ["4h", "9h", "Qd", "4d", "2d"]),
      base,
      street("t", "turn", ["4h", "9h", "Qd", "4d"]),
      street("f", "flop", ["4h", "9h", "Qd"]),
    ]);
    expect(rows.map((r) => r.kind)).toEqual(["hand", "solve"]);
    expect(rows[0].kind === "hand" && rows[0].entries.map((e) => e.job_id)).toEqual(["r", "t", "f"]);
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) =>
      String(input).includes("/library")
        ? Response.json([street("r", "river", ["4h", "9h", "Qd", "4d", "2d"]), street("f", "flop", ["4h", "9h", "Qd"])])
        : Response.json({ error: { code: "x", message: "not mocked" } }, { status: 404 }),
    );
    render(<Library />);
    expect(await screen.findByText("Hand #3")).toBeInTheDocument();
    expect(screen.getByText(/flop, river solved/)).toBeInTheDocument();
  });
});

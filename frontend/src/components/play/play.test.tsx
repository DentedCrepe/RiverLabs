import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PlayReport } from "@/lib/play";
import { ReportView } from "./PlayReport";

const ratio = (made: number, opportunities: number) => ({ made, opportunities });

const REPORT: PlayReport = {
  hands: 40,
  units: "bb",
  net: -3500,
  net_bb: -3.5,
  bb_per_100: -8.75,
  stats: {
    vpip: ratio(10, 40),
    pfr: ratio(6, 40),
    three_bet: ratio(1, 8),
    fold_to_three_bet: ratio(0, 0),
    cbet: ratio(2, 3),
    fold_to_cbet: ratio(1, 2),
    went_to_showdown: ratio(3, 9),
    won_at_showdown: ratio(1, 3),
    aggression_factor: 1.5,
  },
  by_position: [{ position: "BTN", hands: 7, net_bb: 2, vpip: ratio(3, 7) }],
  model_comparison: { decisions: 30, voluntary_you: 0.3, voluntary_model: 0.25, raise_you: 0.2, raise_model: 0.18 },
  all_in: null,
  findings: [
    {
      key: "called_without_odds",
      title: "Called without enough equity (river/all-in)",
      detail: "detail",
      measured_against: "Pot odds vs your equity against the bots' actual ranges",
      count: 2,
      out_of: 5,
      examples: [{ record: 0, hand_no: 12, street: "river", text: "river facing 5bb into 10bb" }],
    },
  ],
  caveats: ["40 hands is a small sample"],
};

describe("session report", () => {
  it("shows statistics as counts and every finding with its reference", () => {
    render(<ReportView report={REPORT} />);
    expect(screen.getByText("25%")).toBeInTheDocument(); // VPIP 10/40
    expect(screen.getByText("2 of 5 decisions")).toBeInTheDocument();
    expect(screen.getByText(/Measured against: Pot odds/)).toBeInTheDocument();
    expect(screen.getByText(/Hand 12: river facing/)).toBeInTheDocument();
    expect(screen.getByText(/small sample/)).toBeInTheDocument();
  });

  it("shows an empty state with no hands", () => {
    render(<ReportView report={{ ...REPORT, hands: 0 }} />);
    expect(screen.getByText("No hands yet")).toBeInTheDocument();
  });
});

import { InfoTip } from "@/components/ui/InfoTip";
import type { PlayState, TableView } from "@/lib/play";
import { PlayTable } from "./PlayTable";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

const seat = (name: string, extra: Partial<TableView["seats"][number]> = {}) => ({
  name,
  is_hero: name === "You",
  position: name === "You" ? "BTN" : "BB",
  is_button: name === "You",
  stack: 97_500,
  street_commit: 0,
  folded: false,
  all_in: false,
  busted: false,
  finish: null,
  cards: name === "You" ? ["As", "Kd"] : null,
  in_hand: true,
  last_action: null,
  hidden_cards: null,
  ...extra,
});

const table = (patch: Partial<TableView>): PlayState => ({
  session: {
    id: "s1",
    format: "cash",
    status: "active",
    error_message: null,
    hands: 3,
    hero_net: 0,
    created_at: "",
    finished_at: null,
    config: {},
    tracked: true,
  },
  table: {
    status: "awaiting_hero",
    format: "cash",
    units: "bb",
    big_blind: 1000,
    small_blind: 500,
    ante: 0,
    level: null,
    hand_no: 4,
    street: "river",
    board: ["Ks", "7d", "2c", "8h", "3d"],
    pot: 10_000,
    seats: [seat("You"), seat("Bot 1 (hard)", { last_action: "bets 5bb", street_commit: 5000 })],
    to_act: 0,
    legal: { fold: true, check: false, call: 5000, bet: null, raise: [10_000, 97_500] },
    log: [{ street: "river", player: "Bot 1 (hard)", text: "bets 5bb" }],
    last_hand_net: null,
    hero_total_net: 0,
    notes: [],
    reveal: false,
    feedback: [
      {
        street: "turn",
        action: "calls",
        feedback: {
          verdict: "mistake",
          headline: "Called with much less equity than needed (flop/turn)",
          detail: "turn facing 5bb into 10bb: needed 25% equity, had 8% vs the range.",
          reference: "Pot odds vs your equity",
        },
      },
    ],
    summary: [],
    hero_equity: 0.31,
    required_equity: 0.25,
    ...patch,
  },
});

describe("play table", () => {
  it("explains a term on tap", async () => {
    render(<InfoTip label="VPIP">Voluntarily put money in pot.</InfoTip>);
    expect(screen.queryByRole("tooltip")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "What is VPIP?" }));
    expect(screen.getByRole("tooltip")).toHaveTextContent("Voluntarily put money in pot.");
  });

  it("shows the bots' last actions, instant feedback and sized actions", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(Response.json(table({})));
    render(<PlayTable state={table({})} onChange={() => undefined} />);
    expect(screen.getByLabelText("Bot 1 (hard) last action")).toHaveTextContent("bets 5bb");
    expect(screen.getByLabelText("Feedback: Flagged")).toHaveTextContent("Called with much less equity");
    await userEvent.click(screen.getByRole("button", { name: "Pot" }));
    // Pot-sized raise facing 5bb into 10bb (the pot includes the bet): call 5 makes the pot 15,
    // so raise to 5 + 15 = 20bb.
    expect(screen.getByRole("button", { name: /Raise to 20bb/ })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Call 5bb/ }));
    const body = JSON.parse(String((fetchSpy.mock.calls[0][1] as RequestInit).body));
    expect(body).toEqual({ action: { type: "call" } });
    fetchSpy.mockRestore();
  });

  it("offers to show unshown hands and the result after a hand", async () => {
    const over = table({
      status: "hand_over",
      to_act: null,
      legal: null,
      last_hand_net: -5000,
      summary: ["Bot 1 (hard) wins 10bb (everyone else folded)"],
      seats: [seat("You"), seat("Bot 1 (hard)", { hidden_cards: ["Qh", "Qc"] })],
    });
    render(<PlayTable state={over} onChange={() => undefined} />);
    expect(screen.getByLabelText("Hand result")).toHaveTextContent("Bot 1 (hard) wins 10bb");
    expect(screen.queryByRole("img", { name: /queen of hearts/i })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Show all hands" }));
    expect(screen.getAllByRole("img").length).toBeGreaterThan(2);
  });
});

import { HandReview } from "./HandHistory";
import type { HandRecord } from "@/lib/play";

describe("hand review", () => {
  it("shows your live equity against the price", () => {
    render(<PlayTable state={table({})} onChange={() => undefined} />);
    expect(screen.getByLabelText("Your odds")).toHaveTextContent("31%");
    expect(screen.getByLabelText("Your odds")).toHaveTextContent("Price needs 25%");
  });

  it("replays a hand street by street with the model's mix for your decisions", () => {
    const hand: HandRecord = {
      hand_no: 7,
      level: null,
      tournament: false,
      small_blind: 500,
      big_blind: 1000,
      ante: 0,
      seats: [
        {
          name: "Bot 1 (hard)",
          position: "SB",
          starting_stack: 100_000,
          hole_cards: ["Qh", "Qc"],
          shown: false,
          is_hero: false,
        },
        { name: "You", position: "BB", starting_stack: 100_000, hole_cards: ["As", "Kd"], shown: false, is_hero: true },
      ],
      board: ["Ks", "7d", "2c"],
      actions: [
        {
          seat: 0,
          street: "preflop",
          action: { type: "raise", to: 2500 },
          added: 2000,
          street_commit: 2500,
          all_in: false,
        },
        { seat: 1, street: "preflop", action: { type: "call" }, added: 1500, street_commit: 2500, all_in: false },
        { seat: 1, street: "flop", action: { type: "check" }, added: 0, street_commit: 0, all_in: false },
        { seat: 0, street: "flop", action: { type: "check" }, added: 0, street_commit: 0, all_in: false },
      ],
      net: [-2500, 2500],
      showdown: false,
      hero_decisions: [
        {
          street: "preflop",
          action: { type: "call" },
          to_call: 1500,
          pot: 3500,
          required_equity: 0.3,
          equity_vs_ranges: 0.55,
          opponents: 1,
          model: {
            spot: "vs_open",
            class: "AKo",
            actions: ["Fold", "Call", "Raise 7.5"],
            frequencies: [0, 0.35, 0.65],
            taken: 1,
          },
          all_in_call: false,
          feedback: { verdict: "ok", headline: "A choice the model also makes (35%)", detail: "…", reference: null },
        },
        {
          street: "flop",
          action: { type: "check" },
          to_call: 0,
          pot: 5000,
          required_equity: null,
          equity_vs_ranges: 0.7,
          opponents: 1,
          model: null,
          all_in_call: false,
          feedback: { verdict: "not_graded", headline: "Not graded", detail: "…", reference: null },
          spot: { street: "flop", board: ["Ks", "7d", "2c"] },
        },
      ],
      hero_all_in_ev: null,
      revealed: false,
      summary: ["You win 5bb (everyone else folded)"],
    };
    const { unmount } = render(<HandReview hand={hand} />);
    expect(screen.queryByRole("link", { name: /Solve this spot/ })).not.toBeInTheDocument();
    unmount();
    render(<HandReview hand={hand} sessionId="s1" />);
    // Only the heads-up postflop decision can be solved.
    expect(screen.getAllByRole("link", { name: /Solve this spot/ })).toHaveLength(1);
    expect(screen.getByRole("link", { name: /Solve this spot/ })).toHaveAttribute(
      "href",
      `/solver?game=s1&hand=${hand.hand_no}&decision=1`,
    );
    expect(screen.getByLabelText("Model frequencies for your hand")).toHaveTextContent("Call ← you");
    expect(screen.getByLabelText("Model frequencies for your hand")).toHaveTextContent("65%");
    expect(screen.getByText("Bot 1 (hard) raises to 2.5bb")).toBeInTheDocument();
    expect(screen.getByText("You call")).toBeInTheDocument();
    expect(screen.getByText(/your equity 55% · price needs 30%/)).toBeInTheDocument();
    expect(screen.getByText("You win 5bb (everyone else folded)")).toBeInTheDocument();
  });
});

describe("table layout", () => {
  it("seats you at the bottom and the others clockwise", async () => {
    const { seatAngle } = await import("./TableFelt");
    expect(seatAngle(0, 6)).toBe(90);
    expect(seatAngle(1, 6)).toBe(150);
    expect(seatAngle(3, 6)).toBe(270);
    expect(seatAngle(1, 2)).toBe(270);
  });
});

describe("flicking through a weakness", () => {
  it("steps through the hands behind a finding", async () => {
    const { FindingBrowser } = await import("./FindingBrowser");
    const { waitFor } = await import("@testing-library/react");
    const record = {
      level: null,
      tournament: false,
      small_blind: 500,
      big_blind: 1000,
      ante: 0,
      seats: [
        {
          name: "Bot 1 (hard)",
          position: "SB",
          starting_stack: 100_000,
          hole_cards: ["7c", "2d"],
          shown: false,
          is_hero: false,
        },
        { name: "You", position: "BB", starting_stack: 100_000, hole_cards: ["As", "Kd"], shown: false, is_hero: true },
      ],
      board: [],
      actions: [],
      net: [500, -500],
      showdown: false,
      hero_decisions: [],
      hero_all_in_ev: null,
      revealed: false,
      summary: [],
    };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const n = Number(String(input).split("/hands/")[1]);
      return Response.json({ ...record, hand_no: n });
    });
    render(
      <FindingBrowser
        examples={[
          { record: 0, hand_no: 4, street: "preflop", text: "BB vs open with 72o", session_id: "s1" },
          { record: 1, hand_no: 9, street: "river", text: "river facing 5bb", session_id: "s2" },
        ]}
      />,
    );
    expect(await screen.findByText(/1 of 2/)).toBeInTheDocument();
    expect(screen.getByText(/Hand 4, preflop: BB vs open with 72o/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Next hand" }));
    expect(await screen.findByText(/Hand 9, river/)).toBeInTheDocument();
    await waitFor(() =>
      expect(fetchSpy).toHaveBeenLastCalledWith(expect.stringContaining("/sessions/s2/hands/9"), expect.anything()),
    );
    expect(screen.getByRole("button", { name: "Next hand" })).toBeDisabled();
  });
});

describe("older hand records", () => {
  it("replays a hand stored before newer fields existed", async () => {
    const { HandReview } = await import("./HandHistory");
    const old = {
      hand_no: 2,
      level: null,
      tournament: false,
      small_blind: 500,
      big_blind: 1000,
      ante: 0,
      seats: [
        {
          name: "Bot 1 (hard)",
          position: "SB",
          starting_stack: 100_000,
          hole_cards: ["7c", "2d"],
          shown: false,
          is_hero: false,
        },
        { name: "You", position: "BB", starting_stack: 100_000, hole_cards: ["As", "Kd"], shown: false, is_hero: true },
      ],
      board: [],
      actions: [],
      net: [500, -500],
      showdown: false,
      hero_decisions: [],
      hero_all_in_ev: null,
    } as unknown as Parameters<typeof HandReview>[0]["hand"];
    render(<HandReview hand={old} />);
    expect(screen.getByLabelText("Players")).toHaveTextContent("You");
  });
});

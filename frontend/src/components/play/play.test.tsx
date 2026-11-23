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
      examples: [{ hand_no: 12, street: "river", text: "river facing 5bb into 10bb" }],
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

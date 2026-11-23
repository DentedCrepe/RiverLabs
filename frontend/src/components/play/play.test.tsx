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

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type DailyPlan, DailyGoalsCard } from "./DailyGoalsCard";

afterEach(() => vi.restoreAllMocks());

const plan: DailyPlan = {
  today: { date: "2026-11-23", hands: 12, study: 10, done: false },
  targets: { hands: 30, study: 10 },
  streak: 4,
  best_streak: 9,
  days: Array.from({ length: 14 }, (_, i) => ({ date: `d${i}`, hands: i, study: i, met: i > 9 })),
  recommended: [
    {
      kind: "weakness",
      title: "Pot odds and break-even equity",
      reason: "Called without enough equity: 3 of 8 decisions in your last 40 cash hands.",
      href: "/academy/pot-odds",
      also: { href: "/analytics", label: "Flick through those hands" },
    },
  ],
};

describe("daily goals", () => {
  it("shows the streak, today's progress, two weeks and what to study", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(Response.json(plan));
    render(<DailyGoalsCard />);
    expect(await screen.findByText("4")).toBeInTheDocument();
    expect(screen.getByText(/day streak · best 9/)).toBeInTheDocument();
    expect(screen.getByText("12 / 30")).toBeInTheDocument();
    expect(screen.getByText("10 / 10")).toBeInTheDocument();
    expect(screen.getByLabelText("Last 14 days").children).toHaveLength(14);
    expect(screen.getByRole("link", { name: "Pot odds and break-even equity" })).toHaveAttribute(
      "href",
      "/academy/pot-odds",
    );
    expect(screen.getByRole("link", { name: "Flick through those hands" })).toBeInTheDocument();
  });

  it("edits the targets", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(async (_input, init) =>
        Response.json(init?.method === "PUT" ? { ...plan, targets: { hands: 50, study: 5 } } : plan),
      );
    render(<DailyGoalsCard />);
    await userEvent.click(await screen.findByRole("button", { name: "Targets" }));
    const hands = screen.getByLabelText("Hands a day");
    await userEvent.clear(hands);
    await userEvent.type(hands, "50");
    const study = screen.getByLabelText("Study items a day");
    await userEvent.clear(study);
    await userEvent.type(study, "5");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("12 / 50")).toBeInTheDocument();
    const put = fetchSpy.mock.calls.find(([, init]) => init?.method === "PUT");
    expect(JSON.parse(String(put?.[1]?.body))).toEqual({ hands_target: 50, study_target: 5 });
  });
});

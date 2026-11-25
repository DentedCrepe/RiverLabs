import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { timeoutFor } from "@/lib/proxy";
import { TutorPanel } from "./TutorPanel";

afterEach(() => {
  vi.restoreAllMocks();
  window.localStorage.clear();
});

const READY = { available: true, reason: null, model: "claude-opus-5-5" };

describe("AI tutor panel", () => {
  it("explains how to set it up when no API key is configured", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      Response.json({ available: false, reason: "no Anthropic API key is set (ANTHROPIC_API_KEY)", model: null }),
    );
    render(<TutorPanel slug="pot-odds" level="simple" />);
    expect(await screen.findByText(/not set up on this RiverLab/)).toHaveTextContent("ANTHROPIC_API_KEY");
  });

  it("asks for consent before anything is sent, then shows answers with their sources", async () => {
    // Shape-only fixture: not a real tutor answer.
    const answer = {
      answer: "You need **25%** equity.",
      sections: ["break-even", "equity#what"],
      readings: ["The Mathematics of Poker"],
      dropped_readings: [],
      suggested_lessons: ["equity"],
      tool_calls: [{ tool: "theory", input: {}, ok: true }],
      unverified_numbers: [],
      corrections: 0,
      model: "claude-opus-5-5",
      label: "AI tutor (may be wrong)",
    };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      if (String(input).endsWith("/tutor") && !String(input).includes("/lessons/")) return Response.json(READY);
      return Response.json(answer);
    });
    render(<TutorPanel slug="pot-odds" level="intermediate" />);
    await userEvent.click(await screen.findByRole("button", { name: /I agree/ }));
    expect(window.localStorage.getItem("riverlab.tutorConsent")).toBe("yes");
    await userEvent.type(screen.getByLabelText("Your question"), "How much equity do I need?");
    await userEvent.click(screen.getByRole("button", { name: "Ask" }));
    const convo = await screen.findByLabelText("Tutor conversation");
    expect(convo).toHaveTextContent("You need 25% equity.");
    const sources = screen.getByLabelText("Tutor sources");
    expect(sources).toHaveTextContent("#break-even");
    expect(sources).toHaveTextContent("equity#what");
    expect(sources).toHaveTextContent("Numbers from RiverLab: theory formula (1 call)");
    expect(sources).toHaveTextContent("Read first: equity");
    const post = fetchSpy.mock.calls.find((c) => String(c[0]).includes("/lessons/pot-odds/tutor"))!;
    const body = JSON.parse(String((post[1] as RequestInit).body));
    expect(body).toMatchObject({ level: "intermediate", question: "How much equity do I need?", consent: true });
    await userEvent.click(screen.getByRole("button", { name: "Turn the tutor off" }));
    expect(screen.getByRole("button", { name: /I agree/ })).toBeInTheDocument();
  });
});

describe("proxy timeouts", () => {
  it("gives tutor answers longer than other requests", () => {
    expect(timeoutFor(["api", "v1", "academy", "lessons", "pot-odds", "tutor"])).toBe(300_000);
    expect(timeoutFor(["api", "v1", "academy", "lessons", "pot-odds"])).toBe(60_000);
  });
});

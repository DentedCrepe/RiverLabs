import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { COMPONENTS } from "./registry";
import { Markdown } from "./Markdown";
import { QuestionCard } from "./QuestionCard";

afterEach(() => vi.restoreAllMocks());

describe("academy components", () => {
  it("registers exactly the components lesson files may use", () => {
    const declared = JSON.parse(
      readFileSync(resolve(__dirname, "../../../../content/academy/components.json"), "utf8"),
    );
    expect(Object.keys(COMPONENTS).sort()).toEqual(Object.keys(declared).sort());
  });

  it("renders math with KaTeX and markdown tables", () => {
    const { container } = render(<Markdown text={"Need $\\frac{B}{P+2B}$.\n\n| a | b |\n|---|---|\n| 1 | 2 |"} />);
    expect(container.querySelector(".katex")).not.toBeNull();
    expect(screen.getByRole("table")).toBeInTheDocument();
  });

  it("submits an answer with confidence and shows the graded explanation", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      Response.json({
        question_id: "pot-odds:half-pot",
        correct: true,
        answer: 25,
        tolerance: 0.5,
        explanation: "10 / 40 = 25%.",
        computed_by_engine: true,
      }),
    );
    render(
      <QuestionCard
        question={{
          id: "pot-odds:half-pot",
          kind: "numeric",
          prompt: "What equity?",
          options: null,
          unit: "%",
          concepts: [],
        }}
      />,
    );
    await userEvent.type(screen.getByLabelText("Answer"), "25");
    await userEvent.click(screen.getByRole("button", { name: "4" }));
    await userEvent.click(screen.getByRole("button", { name: "Check answer" }));
    await waitFor(() => expect(screen.getByText(/Correct/)).toBeInTheDocument());
    expect(screen.getByText("10 / 40 = 25%.")).toBeInTheDocument();
    expect(screen.getByText(/computed by RiverLab/)).toBeInTheDocument();
    const body = JSON.parse(String((fetchSpy.mock.calls[0][1] as RequestInit).body));
    expect(body).toMatchObject({ answer: 25, confidence: 4 });
  });
});

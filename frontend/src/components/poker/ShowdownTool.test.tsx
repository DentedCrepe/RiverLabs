import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ShowdownTool } from "./ShowdownTool";

describe("ShowdownTool", () => {
  it("shows engine results and the winner", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      Response.json({
        results: [
          {
            value: 2,
            category: "one_pair",
            category_name: "One pair",
            description: "One pair, kings",
            best_five: ["Ks", "Kd", "Td", "7h", "4c"],
          },
          {
            value: 3,
            category: "three_of_a_kind",
            category_name: "Three of a kind",
            description: "Three of a kind, tens",
            best_five: ["Tc", "Th", "Td", "Ks", "7h"],
          },
        ],
        winners: [1],
      }),
    );
    render(<ShowdownTool />);
    await userEvent.click(screen.getByRole("button", { name: "Evaluate" }));
    expect(await screen.findByText("Three of a kind, tens")).toBeInTheDocument();
    expect(screen.getByText("Wins")).toBeInTheDocument();
    const body = JSON.parse(String(fetchSpy.mock.calls[0][1]?.body));
    expect(body).toEqual({ board: "KsTd7h4c2s", hands: ["AhKd", "TcTh"] });
  });

  it("displays the engine's rejection message", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      Response.json(
        { error: { code: "engine_validation_error", message: "card Ks appears more than once" } },
        { status: 422 },
      ),
    );
    render(<ShowdownTool />);
    await userEvent.click(screen.getByRole("button", { name: "Evaluate" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("card Ks appears more than once");
  });
});

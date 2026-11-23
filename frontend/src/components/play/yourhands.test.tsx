import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { YourHands } from "./YourHands";

afterEach(() => vi.restoreAllMocks());

describe("YourHands", () => {
  it("filters by verdict and says when nothing matches", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(Response.json({ total: 0, hands: [] }));
    render(<YourHands />);
    expect(await screen.findByText("No hands yet")).toBeInTheDocument();
    fetchSpy.mockResolvedValue(Response.json({ total: 0, hands: [] }));
    await userEvent.click(screen.getByRole("tab", { name: "Flagged" }));
    expect(await screen.findByText("No hands match")).toBeInTheDocument();
    expect(fetchSpy).toHaveBeenLastCalledWith(expect.stringContaining("verdict=mistake"), expect.anything());
  });
});

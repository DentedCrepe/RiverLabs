import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ScopeBanner } from "./PreflopResultView";

describe("preflop result labelling", () => {
  it("labels anything but heads-up push/fold as an approximate model", () => {
    render(<ScopeBanner scope="approximate_model" />);
    expect(screen.getByRole("note")).toHaveTextContent("Approximate model, not a solved game.");
  });

  it("labels validated push/fold as validated, not approximate", () => {
    render(<ScopeBanner scope="heads_up_push_fold" />);
    expect(screen.getByText(/Heads-up push\/fold\. RiverLab has checked/)).toBeInTheDocument();
    expect(screen.queryByText(/Approximate model/)).toBeNull();
  });
});

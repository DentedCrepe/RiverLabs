import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { weightsToNotation } from "./RangeEditor";
import { RangeMatrix } from "./RangeMatrix";

describe("RangeMatrix", () => {
  it("renders 169 cells in matrix order", () => {
    render(<RangeMatrix weights={{}} />);
    const cells = screen.getAllByRole("button");
    expect(cells).toHaveLength(169);
    expect(cells[0]).toHaveAccessibleName(/^AA, 0% weight/);
    expect(cells[1]).toHaveAccessibleName(/^AKs/);
    expect(cells[13]).toHaveAccessibleName(/^AKo/);
    expect(cells[168]).toHaveAccessibleName(/^22/);
  });

  it("paints with the brush and erases a cell already at the brush weight", () => {
    const onPaint = vi.fn();
    const { rerender } = render(<RangeMatrix weights={{}} brush={0.5} onPaint={onPaint} />);
    fireEvent.pointerDown(screen.getByRole("button", { name: /^AKs,/ }));
    expect(onPaint).toHaveBeenLastCalledWith("AKs", 0.5);
    rerender(<RangeMatrix weights={{ AKs: 0.5 }} brush={0.5} onPaint={onPaint} />);
    fireEvent.pointerDown(screen.getByRole("button", { name: /^AKs,/ }));
    expect(onPaint).toHaveBeenLastCalledWith("AKs", 0);
  });

  it("drag-paints across cells", () => {
    const onPaint = vi.fn();
    render(<RangeMatrix weights={{}} onPaint={onPaint} />);
    fireEvent.pointerDown(screen.getByRole("button", { name: /^AA,/ }));
    fireEvent.pointerEnter(screen.getByRole("button", { name: /^KK,/ }));
    fireEvent.pointerUp(window);
    fireEvent.pointerEnter(screen.getByRole("button", { name: /^QQ,/ }));
    expect(onPaint.mock.calls.map((c) => c[0])).toEqual(["AA", "KK"]);
  });

  it("supports keyboard navigation and activation", async () => {
    const onPaint = vi.fn();
    const user = userEvent.setup();
    render(<RangeMatrix weights={{}} onPaint={onPaint} />);
    await user.tab();
    expect(screen.getByRole("button", { name: /^AA,/ })).toHaveFocus();
    await user.keyboard("{ArrowRight}{ArrowDown}");
    expect(screen.getByRole("button", { name: /^KK,/ })).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onPaint).toHaveBeenCalledWith("KK", 1);
  });

  it("is inert when read-only", () => {
    const onPaint = vi.fn();
    render(<RangeMatrix weights={{ AA: 1 }} onPaint={onPaint} readOnly />);
    fireEvent.pointerDown(screen.getByRole("button", { name: /^AA,/ }));
    expect(onPaint).not.toHaveBeenCalled();
  });
});

describe("weightsToNotation", () => {
  it("emits engine notation for non-zero weights", () => {
    expect(weightsToNotation({ AA: 1, AKs: 0.5, KQo: 0 })).toBe("AA,AKs:0.5");
    expect(weightsToNotation({})).toBe("");
  });
});

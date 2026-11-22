import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Button } from "./Button";
import { EmptyState, ErrorState, LoadingState } from "./States";
import { StatusBadge } from "./StatusBadge";

describe("UI primitives", () => {
  it("disables a loading button and marks it busy", async () => {
    const onClick = vi.fn();
    render(
      <Button loading onClick={onClick}>
        Save
      </Button>,
    );
    const btn = screen.getByRole("button", { name: /save/i });
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("aria-busy", "true");
    await userEvent.click(btn);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("labels feature status honestly", () => {
    render(
      <>
        <StatusBadge status="planned" />
        <StatusBadge status="interface_only" />
        <StatusBadge status="partial" />
        <StatusBadge status="available" />
      </>,
    );
    for (const text of ["Planned", "Interface only", "In development", "Available"]) {
      expect(screen.getByText(text)).toBeInTheDocument();
    }
  });

  it("renders accessible states", () => {
    render(
      <>
        <LoadingState label="Loading ranges" />
        <ErrorState message="boom" />
        <EmptyState title="Nothing" description="Planned" />
      </>,
    );
    expect(screen.getByRole("status")).toHaveTextContent("Loading ranges");
    expect(screen.getByRole("alert")).toHaveTextContent("boom");
    expect(screen.getByText("Nothing")).toBeInTheDocument();
  });
});

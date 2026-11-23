import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { usePathname } from "next/navigation";
import { describe, expect, it, vi } from "vitest";
import { MobileNav } from "./MobileNav";
import { NavList } from "./NavList";

describe("NavList", () => {
  it("renders all sections and marks the current page", () => {
    vi.mocked(usePathname).mockReturnValue("/solver");
    render(<NavList />);
    expect(screen.getAllByRole("link")).toHaveLength(10);
    expect(screen.getByRole("link", { name: /solver/i })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: /dashboard/i })).not.toHaveAttribute("aria-current");
  });
});

describe("MobileNav", () => {
  it("opens as a dialog and closes with Escape and on navigation", async () => {
    const user = userEvent.setup();
    render(<MobileNav />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /open navigation/i }));
    expect(screen.getByRole("dialog", { name: /navigation/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /close navigation/i })).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /open navigation/i }));
    await user.click(screen.getByRole("link", { name: /settings/i }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

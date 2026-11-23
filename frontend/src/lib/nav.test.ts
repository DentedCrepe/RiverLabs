import { describe, expect, it } from "vitest";
import { NAV_ITEMS, isActive } from "./nav";

describe("navigation", () => {
  it("contains the ten product areas in order", () => {
    expect(NAV_ITEMS.map((i) => i.label)).toEqual([
      "Dashboard",
      "Preflop",
      "Postflop",
      "Solver",
      "Play",
      "Trainer",
      "Hand Analysis",
      "Analytics",
      "Solution Library",
      "Settings",
    ]);
    expect(new Set(NAV_ITEMS.map((i) => i.href)).size).toBe(10);
  });

  it("labels partial and planned features honestly", () => {
    const status = Object.fromEntries(NAV_ITEMS.map((i) => [i.label, i.status]));
    expect(status.Solver).toBe("partial");
    expect(status.Trainer).toBe("planned");
    expect(status.Play).toBe("approximate");
    expect(status.Preflop).toBe("approximate");
  });

  it("matches active routes by segment", () => {
    expect(isActive("/", "/")).toBe(true);
    expect(isActive("/preflop", "/")).toBe(false);
    expect(isActive("/preflop/x", "/preflop")).toBe(true);
    expect(isActive("/preflopx", "/preflop")).toBe(false);
  });
});

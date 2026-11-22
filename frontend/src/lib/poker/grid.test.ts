import { describe, expect, it } from "vitest";
import { GRID, cellAt } from "./grid";

describe("13x13 grid layout", () => {
  it("has 169 unique labels: 13 pairs, 78 suited, 78 offsuit", () => {
    expect(GRID).toHaveLength(169);
    expect(new Set(GRID.map((c) => c.label)).size).toBe(169);
    const count = (k: string) => GRID.filter((c) => c.kind === k).length;
    expect([count("pair"), count("suited"), count("offsuit")]).toEqual([13, 78, 78]);
  });

  it("matches the engine convention (suited above the diagonal)", () => {
    expect(cellAt(0, 0).label).toBe("AA");
    expect(cellAt(0, 1).label).toBe("AKs");
    expect(cellAt(1, 0).label).toBe("AKo");
    expect(cellAt(11, 12).label).toBe("32s");
    expect(cellAt(12, 12).label).toBe("22");
  });
});

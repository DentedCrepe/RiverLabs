import { describe, expect, it } from "vitest";
import { bbToMbb, formatBb, formatBps, parseScaled, percentToBps } from "./units";

describe("exact unit conversion", () => {
  it("parses bb into integer mbb without floating point error", () => {
    expect(bbToMbb("100")).toBe(100_000);
    expect(bbToMbb("2.5")).toBe(2_500);
    expect(bbToMbb("0.001")).toBe(1);
    expect(bbToMbb("0.1")).toBe(100);
    expect(bbToMbb(" 62.5 ")).toBe(62_500);
  });

  it("rejects input finer than the unit or malformed", () => {
    expect(bbToMbb("0.0005")).toBeNull();
    expect(bbToMbb("-1")).toBeNull();
    expect(bbToMbb("1e3")).toBeNull();
    expect(bbToMbb("")).toBeNull();
    expect(bbToMbb("abc")).toBeNull();
    expect(percentToBps("5.125")).toBeNull();
    expect(parseScaled("99999999999999999999", 3)).toBeNull();
  });

  it("converts percentages to basis points", () => {
    expect(percentToBps("5")).toBe(500);
    expect(percentToBps("4.75")).toBe(475);
  });

  it("formats round-trip exactly", () => {
    for (const mbb of [0, 1, 500, 1_050, 2_500, 100_000, 62_500]) {
      expect(bbToMbb(formatBb(mbb))).toBe(mbb);
    }
    expect(formatBb(1_050)).toBe("1.05");
    expect(formatBps(475)).toBe("4.75");
    expect(formatBps(500)).toBe("5");
  });
});

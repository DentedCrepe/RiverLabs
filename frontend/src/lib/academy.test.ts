import { describe, expect, it } from "vitest";
import { type Block, type ExplanationLevel, type Section, blocksForLevel } from "./academy";

const md = (level: ExplanationLevel | null, text: string): Block => ({ type: "markdown", level, text });

const SECTION: Section = {
  id: "s",
  title: "S",
  blocks: [
    md(null, "shared"),
    md("simple", "easy"),
    md("advanced", "hard"),
    { type: "callout", kind: "fact", text: "fact" },
  ],
};

const texts = (level: Parameters<typeof blocksForLevel>[1]) =>
  blocksForLevel(SECTION, level).map((b) => ("text" in b ? b.text : b.name));

describe("explanation levels", () => {
  it("shows shared blocks and callouts at every level, plus the chosen level's text", () => {
    expect(texts("simple")).toEqual(["shared", "easy", "fact"]);
    expect(texts("advanced")).toEqual(["shared", "hard", "fact"]);
  });

  it("falls back to the closest lower level, then the closest higher one", () => {
    expect(texts("intermediate")).toEqual(["shared", "easy", "fact"]);
    expect(texts("math")).toEqual(["shared", "hard", "fact"]);
    const onlyMath: Section = { id: "m", title: "", blocks: [md("math", "proof")] };
    expect(blocksForLevel(onlyMath, "simple").map((b) => ("text" in b ? b.text : ""))).toEqual(["proof"]);
  });
});

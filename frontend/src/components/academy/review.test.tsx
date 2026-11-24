import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { type ReviewQueue as Queue, changeRequests } from "@/lib/academy";
import { ReviewQueue } from "./ReviewQueue";

const queue: Queue = {
  totals: { to_review: 1, changes_requested: 1, outdated: 0, reviewed: 0 },
  lessons: [
    {
      slug: "pot-odds",
      title: "Pot odds",
      level: 1,
      version: 2,
      status: "published",
      state: "to_review",
      file_notes: "Drafted with AI assistance.",
      questions: 4,
      history: [],
    },
    {
      slug: "equity",
      title: "Equity",
      level: 1,
      version: 3,
      status: "published",
      state: "changes_requested",
      file_notes: "",
      questions: 5,
      history: [
        {
          id: "r1",
          verdict: "changes_requested",
          reviewer: "Sam",
          notes: "Question 2 is unclear",
          version: 3,
          current: true,
          created_at: "2026-11-24T10:00:00Z",
        },
      ],
    },
  ],
};

describe("lesson review page", () => {
  it("lists lessons to review and posts a review with the reviewer's name", async () => {
    const posts: unknown[] = [];
    vi.spyOn(globalThis, "fetch").mockImplementation(async (_url, init) => {
      if (init?.method === "POST") {
        posts.push(JSON.parse(String(init.body)));
        return Response.json({}, { status: 201 });
      }
      return Response.json(queue);
    });
    const user = userEvent.setup();
    render(<ReviewQueue />);
    const card = await screen.findByRole("region", { name: "Review of Pot odds" });
    expect(screen.queryByRole("region", { name: "Review of Equity" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Mark reviewed" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Enter your name first.");
    await user.type(screen.getByLabelText("Your name"), "Sam");
    await user.click(screen.getByRole("button", { name: "Request changes" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Say what should change.");
    await user.click(screen.getByRole("button", { name: "Mark reviewed" }));
    await vi.waitFor(() => expect(posts).toEqual([{ verdict: "reviewed", reviewer: "Sam", notes: "" }]));
    expect(card).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: /Changes requested/ }));
    expect(screen.getByRole("region", { name: "Review of Equity" })).toHaveTextContent("Question 2 is unclear");
  });

  it("collects the change requests as text", () => {
    expect(changeRequests(queue)).toBe("Equity (equity, v3), from Sam:\nQuestion 2 is unclear");
  });
});

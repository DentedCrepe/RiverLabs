import { describe, expect, it, vi } from "vitest";
import { ApiError, api } from "./api";

function mockFetch(status: number, body: unknown) {
  return vi.spyOn(globalThis, "fetch").mockResolvedValue(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    }),
  );
}

describe("api client", () => {
  it("calls the same-origin proxy and returns JSON", async () => {
    const f = mockFetch(200, { ok: 1 });
    await expect(api.post("/api/v1/x", { a: 1 })).resolves.toEqual({ ok: 1 });
    expect(f).toHaveBeenCalledWith(
      "/api/backend/api/v1/x",
      expect.objectContaining({ method: "POST", body: '{"a":1}' }),
    );
  });

  it("surfaces engine errors with code and message", async () => {
    mockFetch(422, { error: { code: "engine_validation_error", message: "duplicate card As" } });
    const err = await api.get("/x").catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toMatchObject({ status: 422, code: "engine_validation_error", message: "duplicate card As" });
  });

  it("includes request validation details", async () => {
    mockFetch(422, {
      error: {
        code: "request_validation_error",
        message: "Invalid request",
        details: [{ loc: ["body", "cards"], msg: "Field required", type: "missing" }],
      },
    });
    await expect(api.get("/x")).rejects.toThrow("Invalid request: cards Field required");
  });

  it("reports network failures instead of swallowing them", async () => {
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new TypeError("Failed to fetch"));
    await expect(api.get("/x")).rejects.toMatchObject({ code: "network_error" });
  });

  it("handles 204 responses", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(null, { status: 204 }));
    await expect(api.delete("/x")).resolves.toBeUndefined();
  });
});

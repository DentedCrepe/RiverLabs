import { describe, expect, it, vi } from "vitest";
import { forwardToBackend, isAllowedPath } from "./proxy";

describe("backend proxy", () => {
  it("allows only health and v1 API paths with safe segments", () => {
    expect(isAllowedPath(["health"])).toBe(true);
    expect(isAllowedPath(["health", "ready"])).toBe(true);
    expect(isAllowedPath(["api", "v1", "game-configs", "abc-123"])).toBe(true);
    expect(isAllowedPath(["docs"])).toBe(false);
    expect(isAllowedPath(["api", "v2", "x"])).toBe(false);
    expect(isAllowedPath(["api", "v1", ".."])).toBe(false);
    expect(isAllowedPath(["api", "v1", "a b"])).toBe(false);
    // Academy question ids contain a colon.
    expect(isAllowedPath(["api", "v1", "academy", "questions", "pot-odds:half-pot", "attempts"])).toBe(true);
    expect(isAllowedPath([])).toBe(false);
  });

  it("forwards method, body, query and content type only", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(Response.json({ ok: true }, { status: 201 }));
    const req = new Request("http://frontend/api/backend/api/v1/equity?x=1", {
      method: "POST",
      body: '{"a":1}',
      headers: { "content-type": "application/json", cookie: "secret=1", authorization: "Bearer t" },
    });
    const res = await forwardToBackend(req, ["api", "v1", "equity"], "http://backend:8000/", fetchImpl);
    expect(res.status).toBe(201);
    const [url, init] = fetchImpl.mock.calls[0];
    expect(String(url)).toBe("http://backend:8000/api/v1/equity?x=1");
    expect(init.method).toBe("POST");
    expect(new TextDecoder().decode(init.body)).toBe('{"a":1}');
    expect(init.headers).toEqual({ "content-type": "application/json" });
  });

  it("forwards Academy answers, whose question ids contain a colon", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(Response.json({ correct: true }));
    const req = new Request("http://frontend/api/backend/api/v1/academy/questions/pot-odds%3Ahalf-pot/attempts", {
      method: "POST",
      body: '{"answer":25}',
      headers: { "content-type": "application/json" },
    });
    const res = await forwardToBackend(
      req,
      ["api", "v1", "academy", "questions", "pot-odds:half-pot", "attempts"],
      "http://b",
      fetchImpl,
    );
    expect(res.status).toBe(200);
    expect(String(fetchImpl.mock.calls[0][0])).toBe("http://b/api/v1/academy/questions/pot-odds%3Ahalf-pot/attempts");
  });

  it("passes on the background-polling marker so polls don't count as activity", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(Response.json({}));
    const req = new Request("http://frontend/api/backend/api/v1/solver/jobs", { headers: { "x-riverlab-poll": "1" } });
    await forwardToBackend(req, ["api", "v1", "solver", "jobs"], "http://b", fetchImpl);
    expect(fetchImpl.mock.calls[0][1].headers).toEqual({ "x-riverlab-poll": "1" });
  });

  it("returns structured errors for misconfiguration, unknown paths and outages", async () => {
    const req = new Request("http://frontend/api/backend/health");
    expect((await forwardToBackend(req, ["health"], undefined)).status).toBe(500);
    expect((await forwardToBackend(req, ["admin"], "http://b")).status).toBe(404);
    const down = vi.fn().mockRejectedValue(new TypeError("ECONNREFUSED"));
    const res = await forwardToBackend(req, ["health"], "http://b", down);
    expect(res.status).toBe(502);
    expect(await res.json()).toEqual({
      error: { code: "backend_unreachable", message: "The RiverLab API is unreachable" },
    });
  });

  it("rejects oversized bodies by declared length without reading them", async () => {
    const req = new Request("http://f/api/backend/api/v1/x", {
      method: "POST",
      body: "x",
      headers: { "content-length": String(10 * 1024 * 1024) },
    });
    const fetchImpl = vi.fn();
    expect((await forwardToBackend(req, ["api", "v1", "x"], "http://b", fetchImpl)).status).toBe(413);
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it("stops reading a streamed body without Content-Length once it exceeds the limit", async () => {
    let pulled = 0;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        pulled += 1;
        controller.enqueue(new Uint8Array(64 * 1024));
        if (pulled > 1000) controller.close();
      },
    });
    const req = new Request("http://f/api/backend/api/v1/x", {
      method: "POST",
      body: stream,
      duplex: "half",
    } as RequestInit);
    const fetchImpl = vi.fn();
    expect((await forwardToBackend(req, ["api", "v1", "x"], "http://b", fetchImpl)).status).toBe(413);
    expect(pulled).toBeLessThan(10);
    expect(fetchImpl).not.toHaveBeenCalled();
  });
});

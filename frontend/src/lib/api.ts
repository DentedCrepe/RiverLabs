/**
 * Browser API client. All requests go to the same-origin proxy (/api/backend/*, ADR-0006).
 * Backend errors arrive as {"error": {"code", "message", "details"}} and are surfaced as
 * ApiError — never swallowed.
 */

export const API_BASE = "/api/backend";

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
    public readonly details?: unknown,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function parseError(res: Response): Promise<ApiError> {
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    // non-JSON error body
  }
  const err = (body as { error?: { code?: string; message?: string; details?: unknown } } | null)?.error;
  if (err?.message) {
    const detail =
      err.code === "request_validation_error" && Array.isArray(err.details)
        ? `: ${(err.details as { loc: unknown[]; msg: string }[])
            .map((d) => `${d.loc.slice(1).join(".")} ${d.msg}`)
            .join("; ")}`
        : "";
    return new ApiError(res.status, err.code ?? "error", err.message + detail, err.details);
  }
  return new ApiError(res.status, "http_error", `Request failed with HTTP ${res.status}`);
}

export async function apiFetch<T>(path: string, init?: RequestInit & { json?: unknown }): Promise<T> {
  const { json, headers, ...rest } = init ?? {};
  let res: Response;
  try {
    res = await fetch(`${API_BASE}${path}`, {
      ...rest,
      headers: json !== undefined ? { "Content-Type": "application/json", ...headers } : headers,
      body: json !== undefined ? JSON.stringify(json) : rest.body,
    });
  } catch (e) {
    throw new ApiError(0, "network_error", `Network error: ${(e as Error).message}`);
  }
  if (!res.ok) throw await parseError(res);
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  /** `poll`: a background refresh, which the API does not count as user activity. */
  get: <T>(path: string, opts: { poll?: boolean } = {}) =>
    apiFetch<T>(path, opts.poll ? { headers: { "X-RiverLab-Poll": "1" } } : undefined),
  post: <T>(path: string, json: unknown) => apiFetch<T>(path, { method: "POST", json }),
  delete: (path: string) => apiFetch<void>(path, { method: "DELETE" }),
};

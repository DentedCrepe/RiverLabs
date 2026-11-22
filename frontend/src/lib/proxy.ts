const ALLOWED_PREFIXES = [["health"], ["api", "v1"]];
const SEGMENT = /^[A-Za-z0-9._-]+$/;
const MAX_BODY_BYTES = 256 * 1024;
const TIMEOUT_MS = 60_000;

function errorResponse(status: number, code: string, message: string): Response {
  return Response.json({ error: { code, message } }, { status });
}

/** Reads the body as a stream, aborting as soon as it exceeds `limit` bytes. */
async function readCapped(request: Request, limit: number): Promise<Uint8Array<ArrayBuffer> | null> {
  if (!request.body) return new Uint8Array();
  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > limit) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const out = new Uint8Array(total);
  let offset = 0;
  for (const c of chunks) {
    out.set(c, offset);
    offset += c.byteLength;
  }
  return out;
}

export function isAllowedPath(path: string[]): boolean {
  if (path.length === 0 || path.some((s) => !SEGMENT.test(s) || s === "." || s === "..")) return false;
  return ALLOWED_PREFIXES.some((prefix) => prefix.every((p, i) => path[i] === p));
}

export async function forwardToBackend(
  request: Request,
  path: string[],
  baseUrl: string | undefined,
  fetchImpl: typeof fetch = fetch,
): Promise<Response> {
  if (!baseUrl) {
    return errorResponse(500, "proxy_misconfigured", "RIVERLAB_API_URL is not configured on the frontend server");
  }
  if (!isAllowedPath(path)) {
    return errorResponse(404, "not_found", "Unknown API path");
  }
  const incoming = new URL(request.url);
  const target = new URL(`${baseUrl.replace(/\/+$/, "")}/${path.join("/")}`);
  target.search = incoming.search;

  let body: Uint8Array<ArrayBuffer> | undefined;
  if (request.method !== "GET" && request.method !== "HEAD") {
    const declared = Number(request.headers.get("content-length") ?? "0");
    if (declared > MAX_BODY_BYTES) {
      return errorResponse(413, "payload_too_large", "Request body is too large");
    }
    const read = await readCapped(request, MAX_BODY_BYTES);
    if (read === null) return errorResponse(413, "payload_too_large", "Request body is too large");
    body = read;
  }
  const headers: Record<string, string> = {};
  const contentType = request.headers.get("content-type");
  if (contentType) headers["content-type"] = contentType;

  try {
    const res = await fetchImpl(target, {
      method: request.method,
      headers,
      body,
      cache: "no-store",
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
    return new Response(res.body, {
      status: res.status,
      headers: { "content-type": res.headers.get("content-type") ?? "application/json" },
    });
  } catch (e) {
    const timedOut = e instanceof DOMException && e.name === "TimeoutError";
    return errorResponse(
      timedOut ? 504 : 502,
      timedOut ? "backend_timeout" : "backend_unreachable",
      timedOut ? "The API did not respond in time" : "The RiverLab API is unreachable",
    );
  }
}

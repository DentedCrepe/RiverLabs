/**
 * Same-origin proxy to the FastAPI backend (ADR-0006).
 *
 * Only allow-listed path prefixes are forwarded, segments are validated, and only the
 * Content-Type header is passed through. The backend URL is read at request time from
 * RIVERLAB_API_URL, so one build works in every environment.
 */
import { forwardToBackend } from "@/lib/proxy";

type Ctx = { params: Promise<{ path: string[] }> };

async function handle(request: Request, ctx: Ctx): Promise<Response> {
  const { path } = await ctx.params;
  return forwardToBackend(request, path, process.env.RIVERLAB_API_URL);
}

export const GET = handle;
export const POST = handle;
export const DELETE = handle;

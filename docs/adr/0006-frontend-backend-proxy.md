# ADR-0006: Same-origin proxy from Next.js to FastAPI

Status: Accepted (2026-11-22)

The browser calls `/api/backend/*` on the Next.js origin; a route handler forwards to
`RIVERLAB_API_URL` (read at runtime). This avoids CORS configuration, keeps the backend URL out
of the client bundle, and lets the same image run in any environment. The backend still sends
no permissive CORS headers by default.

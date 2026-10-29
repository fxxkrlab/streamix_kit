# Errors and retries

SXAPI errors use `{"error":{"code":"…","message":"…"}}`. The synchronized specification is authoritative. The SDK returns the HTTP status plus a bounded safe `error.code`; it deliberately omits raw bodies, request URLs, headers and network error strings. Do not treat a 200 JSON response as actual player playback.

| Status | Client action |
| --- | --- |
| 400 | Fix input, range or cursor; do not repeat the same invalid request. |
| 401 | Stop using the failed credential; sign in or obtain a new permitted ticket through the proper original login. |
| 403 | Current policy denies access; keep quota, adult/library access and concurrency checks intact. |
| 404 | Unknown or hidden resource; do not reveal another user's resource existence. |
| 409 | Re-read current session/source or restart the catalog page; never invent progress or silently choose a different file. |
| 416 | Validate file representation and requested byte range. |
| 429 | Respect rate/concurrency limits; show an actionable state instead of repeatedly preparing sessions. |
| 503 | Temporary dependency/service failure, not a bad password or lack of library permission. |

The SDK does not blindly retry login, preparation, progress, ticket issue or session termination. The player must decide bounded retries for safe reads. A ticket renewal request may fail after logout, expiry, source change or revocation; none permits a permissive fallback.

Local SDK errors distinguish invalid origin/path, unsupported header contract, malformed response, oversized body, range mismatch and changed representation. Display a safe summary; never print a token-bearing structure in a bug report.

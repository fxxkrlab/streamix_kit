# Synchronized contract

Specification bytes have not been synchronized yet. `SOURCE.json` remains explicitly unbound until the combined server commit, release candidate and generated header-only contract are reviewed.

`tools/sync_spec.py` synchronizes `sxapi-v1.openapi.json` (full published contract) and `sxapi-v1-client.openapi.json` (the exact server-generated player subset), records SHA256 and upstream commit, and generates the operation and SDK-shape inventory. Do not copy a historical URL-ticket contract or hand-edit synchronized files.

The player subset and the Rust SDK do not implement every administration operation in the full specification.

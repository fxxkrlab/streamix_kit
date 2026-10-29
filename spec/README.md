# Synchronized contract

The exact reviewed full/client pair is synchronized from server source `29f600eb47f0f2f893039b654e3b9f7d782576d7`, with minimum candidate 0.9.0-rc.9. `SOURCE.json` is BOUND to those bytes, with no published release reference and compatibility still NOT_VALIDATED. Source synchronization is not SDK CI or integration acceptance.

`tools/sync_spec.py` synchronizes `sxapi-v1.openapi.json` (full API contract) and `sxapi-v1-client.openapi.json` (the exact server-generated player subset), records SHA256 and upstream commit, and generates the operation and SDK-shape inventory. Do not copy a historical URL-ticket contract or hand-edit synchronized files.

The player subset and the Rust SDK do not implement every administration operation in the full specification.

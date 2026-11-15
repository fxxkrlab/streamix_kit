# Synchronized contract

The player subset `sxapi-v1-client.openapi.json` is synchronized from server source `c80602d58c0d7b509b3fe2eb282c3e6f65b4ae7a` (intended 0.9.0-rc.18), with minimum candidate 0.9.0-rc.9. `SOURCE.json` is BOUND to those bytes, with no published release reference and compatibility still NOT_VALIDATED. Source synchronization is not SDK CI or integration acceptance.

Only the player subset is published here (decided 2026-11-15). `tools/sync_spec.py` still takes the reviewed full export as an input to prove that every subset operation and component is an exact copy of the server contract, records the full export's SHA256 and size in `SOURCE.json` (`upstream_full_export`, `published: false`) and writes only the subset, the operation and SDK-shape inventory and `SOURCE.json`. Administration operations are not part of this repository. Do not copy a historical URL-ticket contract or hand-edit synchronized files.

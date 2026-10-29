# Version and compatibility

Kit version: **0.1.0-rc.1**. Build: **20261029.001**. Protocol: **SXAPI major 1**, base `/sxapi/v1`.

The intended minimum compatible server candidate is **0.9.0-rc.9**, with the exact final upstream commit still **unbound** in [SOURCE.json](../spec/SOURCE.json). This is a target, not validated compatibility or a publication claim. This candidate depends on the new header-only video and subtitle contract, not the old URL-ticket transport. Do not advertise compatibility with RC8 or a historical server from a version string alone.

Before usage require runtime capabilities `playback_ticket_header=true`, `credentials_in_urls=false` and the needed individual features. Missing capability keys are unsupported. No lenient legacy fallback is provided. A deployment can disable features even when the protocol/schema is present.

Kit uses its own SemVer. Pre-release server integrations use a pre-release Kit until formally accepted; a final server release does not automatically finalize this Kit. Breaking SXAPI changes require a new protocol major path and a corresponding Kit major. Compatible additive fields may be retained as unknown fields by typed response models.

`sync_spec.py` records the full upstream commit, release, minimum server version, synchronized timestamp and SHA256 of the exact full and player-subset bytes. Both must come from the same reviewed generation. CI rejects local edits, source drift and a subset that differs from the server generation.

Delivery stages remain distinct: source, static review, server CI/mock tests, real test-server flow, Draft PR review, merge, release, SDK distribution and real-player decoding. Each claim needs its own evidence; not-run is not passing.

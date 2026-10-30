# Version and compatibility

Kit version: **0.1.0-rc.1**. Build: **20261030.005**. Protocol: **SXAPI major 1**, base `/sxapi/v1`.

The minimum server source requirement is candidate **0.9.0-rc.9**, bound to upstream commit `2b069a969f6e0ba8c45bc404671b740c1f057565` in [SOURCE.json](../spec/SOURCE.json). The full/client SHA256 values identify the exact synchronized bytes. No upstream release reference is supplied because this candidate is unpublished; compatibility remains NOT_VALIDATED. Actual server API validation ran on `2db3276aa025f78145324e5e4b75a07c6033cd6f`; the bound child only updates its review manifest and leaves both specification files identical. Source binding is not SDK CI or integration acceptance. This candidate depends on the new header-only video and subtitle contract, not the old URL-ticket transport. Do not advertise compatibility with RC8 or a historical server from a version string alone.

Before usage require runtime capabilities `playback_ticket_header=true`, `credentials_in_urls=false` and the needed individual features. The server also declares the additive aliases `stream_ticket_header=true` and `stream_query_ticket=false`; these do not enable a URL transport or change the SDK checks. Missing capability keys are unsupported. No lenient legacy fallback is provided. A deployment can disable features even when the protocol/schema is present.

Kit uses its own SemVer. Pre-release server integrations use a pre-release Kit until formally accepted; a final server release does not automatically finalize this Kit. Breaking SXAPI changes require a new protocol major path and a corresponding Kit major. Compatible additive fields may be retained as unknown fields by typed response models.

`sync_spec.py` records the full upstream commit, release, minimum server version, synchronized timestamp and SHA256 of the exact full and player-subset bytes. Both must come from the same reviewed generation. CI rejects local edits, source drift and a subset that differs from the server generation.

Delivery stages remain distinct: source, static review, server CI/mock tests, real test-server flow, Draft PR review, merge, release, SDK distribution and real-player decoding. Each claim needs its own evidence; not-run is not passing.

Build 20261030.005 synchronizes only specifications, provenance and documentation. Rust sources, Cargo inputs and the generated operation/typed-schema inventory are unchanged. Retain the designated-server 21-test and 1 MiB mock CLI evidence under its actual source `c8ee971b6660418bd4cb2836be0286fab76d29b6` / Build 20261029.004. No Build 20261030.005 SDK binary was compiled; current specification checks await the designated server.

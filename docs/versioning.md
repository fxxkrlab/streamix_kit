# Version and compatibility

Kit version: **0.1.0-rc.1**. Build: **20261110.006**. Protocol: **SXAPI major 1**, base `/sxapi/v1`.

The minimum server source requirement is candidate **0.9.0-rc.9**. The specifications are bound to upstream commit `b39a328f245471d7746542619d024c3ce768266a` (intended server version 0.9.0-rc.10) in [SOURCE.json](../spec/SOURCE.json). The full/client SHA256 values identify the exact synchronized bytes. No upstream release reference is supplied because this candidate is unpublished; compatibility remains NOT_VALIDATED. The bound commit itself passed the server API documentation checks. Source binding is not SDK CI or integration acceptance. This candidate depends on the new header-only video and subtitle contract, not the old URL-ticket transport. Do not advertise compatibility with RC8 or a historical server from a version string alone.

The RC10 specifications differ from RC9 only in `info.version` and two operation descriptions: cross-session progress ordering for `PUT /playback/sessions/{id}/progress` and played/position fencing for `PUT /users/me/media/{id}/state`. No path, parameter, field, capability or typed SDK schema changed, and the SDK does not depend on the new ordering, so the minimum stays 0.9.0-rc.9. Players that need the RC10 progress behavior should connect to an RC10 server.

Before usage require runtime capabilities `playback_ticket_header=true`, `credentials_in_urls=false` and the needed individual features. The server also declares the additive aliases `stream_ticket_header=true` and `stream_query_ticket=false`; these do not enable a URL transport or change the SDK checks. Missing capability keys are unsupported. No lenient legacy fallback is provided. A deployment can disable features even when the protocol/schema is present.

Kit uses its own SemVer. Pre-release server integrations use a pre-release Kit until formally accepted; a final server release does not automatically finalize this Kit. Breaking SXAPI changes require a new protocol major path and a corresponding Kit major. Compatible additive fields may be retained as unknown fields by typed response models.

`sync_spec.py` records the full upstream commit, release, minimum server version, synchronized timestamp and SHA256 of the exact full and player-subset bytes. Both must come from the same reviewed generation. CI rejects local edits, source drift and a subset that differs from the server generation.

Delivery stages remain distinct: source, static review, server CI/mock tests, real test-server flow, Draft PR review, merge, release, SDK distribution and real-player decoding. Each claim needs its own evidence; not-run is not passing.

Build 20261110.006 synchronizes only specifications, provenance and documentation. Rust sources, Cargo inputs and the generated operation/typed-schema inventory are unchanged. Retain the designated-server 21-test and 1 MiB mock CLI evidence under its actual source `c8ee971b6660418bd4cb2836be0286fab76d29b6` / Build 20261029.004. No Build 20261110.006 SDK binary was compiled; its static specification checks passed on a workstation and designated-server checks are pending.

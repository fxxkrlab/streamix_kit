# Changelog

## 0.1.0-rc.1 — unreleased

- Build 20261030.005 synchronizes the reviewed full/client specifications from server `2b069a969f6e0ba8c45bc404671b740c1f057565`, the review-manifest-only child of API-validated `2db3276aa025f78145324e5e4b75a07c6033cd6f`. Add the server capability aliases `stream_ticket_header=true` and `stream_query_ticket=false`; retain `playback_ticket_header=true` and `credentials_in_urls=false`. No URL credential fallback is introduced.
- Rust sources, Cargo inputs and generated operation/typed-schema inventory are unchanged. Retain the original designated-server 21-test and 1 MiB mock CLI results under `c8ee971b6660418bd4cb2836be0286fab76d29b6` / Build 20261029.004. No Build 20261030.005 SDK binary was compiled; its six specification checks await the designated server. Historical build notes below describe their stage at that time, not the current validation status.

- Build 20261029.004 uses eq_ignore_ascii_case for the mock server's required-header comparison, preserving ASCII case-insensitive matching and addressing the designated-server strict Clippy diagnostic without a lint allowance. This is a source-only correction; strict Clippy and downstream checks require the designated-server rerun.
- Build 20261029.003 groups the login fixture's concatenation expression for `json!`; its synthetic token value is unchanged. This addresses the first designated-server cargo-check macro error. The subsequent designated-server cargo check passed for that build.
- Freeze Cargo.lock exactly as produced by the first designated-server dependency resolver (exit 0), with Cargo manifests unchanged. Resolver success is not compilation or test success.
- Prepare the public repository, proprietary integration license, protocol guides, header-only curl example and spec synchronization checks.
- Add a Rust SDK and CLI source candidate for login, browsing, original-file playback, bounded Range reads, ticket renewal, progress and session cleanup.
- Require the new header-only ticket server contract. Build 20261029.002 binds the exact full/client pair to server source `29f600eb47f0f2f893039b654e3b9f7d782576d7`, with minimum candidate 0.9.0-rc.9 and no published release reference.
- Add build/client/device headers, per-install UUIDs, tracing, separate prefetch labels, explicit protocol/port parsing and public-server identity models. Document the RC.9 discovery target; UDP client networking is not included.
- TypeScript, Swift and Kotlin are deferred. Server CI, real test-server acceptance, Draft PR review and publication are not complete.
- Preserve validated Retry-After errors, carry prefetch purpose through short-ticket renewal, and enforce the complete client-label length bound. Regression sources are added; execution remains pending server CI.
- Synchronize SOURCE.json and the operation/schema inventory using the existing sync tool on the designated server. Source synchronization succeeded; this is not SDK compilation, tests, CI, integration or real-player acceptance.

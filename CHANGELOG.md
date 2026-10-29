# Changelog

## 0.1.0-rc.1 — unreleased

- Prepare the public repository, proprietary integration license, protocol guides, header-only curl example and spec synchronization checks.
- Add a Rust SDK and CLI source candidate for login, browsing, original-file playback, bounded Range reads, ticket renewal, progress and session cleanup.
- Require the new header-only ticket server contract. Build 20261029.002 binds the exact full/client pair to server source `29f600eb47f0f2f893039b654e3b9f7d782576d7`, with minimum candidate 0.9.0-rc.9 and no published release reference.
- Add build/client/device headers, per-install UUIDs, tracing, separate prefetch labels, explicit protocol/port parsing and public-server identity models. Document the RC.9 discovery target; UDP client networking is not included.
- TypeScript, Swift and Kotlin are deferred. Server CI, real test-server acceptance, Draft PR review and publication are not complete.
- Preserve validated Retry-After errors, carry prefetch purpose through short-ticket renewal, and enforce the complete client-label length bound. Regression sources are added; execution remains pending server CI.
- Synchronize SOURCE.json and the operation/schema inventory using the existing sync tool on the designated server. Source synchronization succeeded; this is not SDK compilation, tests, CI, integration or real-player acceptance.

# Changelog

## 0.1.0-rc.1 — unreleased

- Build 20261110.006 按服务器提交 `b39a328f245471d7746542619d024c3ce768266a`（目标版本 0.9.0-rc.10，尚未正式发布）同步完整规范和播放器子集。相对 RC9 只有 `info.version` 和两个操作的说明变化：`PUT /users/me/media/{id}/state` 写明显式修改已看/进度会挡住同一用户、同一条目更早的播放上报，只改收藏不影响播放归属；`PUT /playback/sessions/{id}/progress` 写明服务器按条目归属给跨会话的上报排序，被取代或未绑定的旧上报只留诊断审计，不覆盖更新的会话或手动修改。路径、参数、字段、能力声明和 SDK 类型指纹都没有变化，`OPERATIONS.json` 字节不变。
- 最低服务器版本保持 0.9.0-rc.9：线上契约没有变化，SDK 也不依赖新的排序语义。需要 RC10 进度行为的播放器应连接 RC10 服务器。
- Rust 源码、Cargo 输入和 SDK 版本 0.1.0-rc.1 不变，没有编译新的 SDK 二进制；21 项 Rust 测试和 1 MiB mock CLI 证据仍属 `c8ee971b6660418bd4cb2836be0286fab76d29b6` / Build 20261029.004。本机通过规范、类型契约、凭据、链接和两份 OpenAPI 3.1 静态检查；指定服务器检查、真实播放器验收和发布都未执行。

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

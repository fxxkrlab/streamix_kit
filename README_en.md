# StreamixKit

SXAPI v1 integration documents and an asynchronous Rust SDK for player authors.

**0.1.0-rc.2 · Build 20261111.007 — specification-sync candidate, not released.**

The contracts are bound to server source `205a05937ea19b390e62770f87e126b3fec4578a` (0.9.0-rc.11 candidate, not yet published), with minimum server candidate **0.9.0-rc.9**; see [SOURCE.json](spec/SOURCE.json). The RC11 specifications are additive over RC10: the player subset grows from 70 to 77 operations (`GET /users/me/resume`, `GET /users/me/next-up`, `GET /media/initials`, plus POST forms of logout, session end, credential revocation and stream termination), capabilities add `features.list_user_state`, `features.watch_queues`, `features.media_initials` and `features.subtitle_match`, lists and details add `user_state`, `external_ids.tvdb` and `has_logo`, subtitles add `match` and `other_version`, `/media` adds external-ID, initial-letter, multi-genre, in-progress, sort and `include_total` options, and JSON reads support ETag/304. The only typed SDK change: `GET /media` with `include_total=false` returns `total: null`, so `Page.total` becomes `Option<i64>`; the SDK's own requests never send that parameter and still receive a number. The SDK uses no other new operation, so the minimum server candidate stays **0.9.0-rc.9**. The SDK still requires `features.playback_ticket_header=true` and `features.credentials_in_urls=false`. The server also exposes the additive aliases `features.stream_ticket_header=true` and `features.stream_query_ticket=false`. Protocol version `1.0` alone is insufficient; there is no credential-URL fallback.

Build 20261111.007 changes Rust source (`Page.total`) and adds one test, so the SDK version moves to 0.1.0-rc.2. On a workstation, 22 Rust tests and strict Clippy passed, as did the specification, typed-contract, credential and link checks; designated-server checks, real-player acceptance and publication have not run. The original designated-server evidence for Build 20261029.004 (source `c8ee971b6660418bd4cb2836be0286fab76d29b6`: 21 Rust tests and a 1 MiB mock CLI flow) belongs to that source only.

Start with the [quickstart](docs/quickstart.md), [header rules](docs/credentials-in-headers.md), [playback guide](docs/playback.md), [Rust SDK](sdks/rust/README.md), [curl example](examples/curl/README.md), or [Rust CLI](examples/rust-cli/README.md). The [versioning guide](docs/versioning.md) describes compatibility and acceptance status.

[Connection guidance](docs/connecting.md), [discovery guidance](docs/discovery.md) and [client request headers](docs/client-request-headers.md) define trusted origins, local-HTTP opt-in, ports, metadata, per-install UUIDs, purpose and tracing. UDP client networking is not implemented here; deployment and real-LAN acceptance remain separate.

The Rust candidate models login, browsing, versions, prepared playback sessions, ticket renewal, a bounded byte range, progress and explicit session termination. Video requests use `X-SXAPI-Stream-Ticket`; subtitles additionally retain the original login Bearer. Streamix Server currently serves the original file without transcoding. Players must declare their actual supported formats.

TypeScript, Swift and Kotlin SDKs are unimplemented future work. No packages for those languages are advertised or published.

This public repository contains client contracts and integration source only. Specifications are synchronized from the server-generated full and client specifications and must not be edited here.

[Commercial proprietary license](LICENSE): publicly readable source and authorized SDK/API integration, not an open-source license. SDK binary redistribution requires separate permission. Calling the HTTP API does not require SDK redistribution. Third-party dependency licenses remain applicable; see [NOTICES](NOTICES.md).

Tracked in [GitHub #1](https://github.com/fxxkrlab/streamix_kit/issues/1) and [FXX-566](https://linear.app/fxxkrlab/issue/FXX-566). Source availability does not imply compilation, integration, publication or successful playback acceptance.

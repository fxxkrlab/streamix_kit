# StreamixKit

SXAPI v1 integration documents and an asynchronous Rust SDK for player authors.

**0.1.0-rc.2 · Build 20261111.008 — specification-sync candidate, not released.**

The contracts are bound to server source `39ef59f62d4f2a0bdeb477cac885c4764f04fbd6` (0.9.0-rc.12 candidate, not yet published), with minimum server candidate **0.9.0-rc.9**; see [SOURCE.json](spec/SOURCE.json). The RC12 specifications are additive over RC11: the player subset grows from 77 to 79 operations (`GET /persons` person search and `GET /episodes` episode search), capabilities add `features.media_quality_filters`, `features.collection_quality_summary`, `features.person_search` and `features.episode_search` plus `limits.search_candidates_max` and `limits.search_page_size_max`, `GET /media` and `GET /media/initials` add the `official_ratings`, `resolutions`, `dynamic_ranges`, `audio_formats`, `audio_languages` and `subtitle_languages` filters, facets add those six kinds plus years, and collection member resources add `quality_summary`. No typed SDK schema, Rust source or Cargo input changes; the SDK uses no new operation, so the minimum server candidate stays **0.9.0-rc.9**. Players that need search, the new filters or collection quality summaries should connect to an RC12 server and check the matching capability. Since RC11, `GET /media` with `include_total=false` returns `total: null`, so `Page.total` is `Option<i64>`; the SDK's own requests never send that parameter. The SDK still requires `features.playback_ticket_header=true` and `features.credentials_in_urls=false`. The server also exposes the additive aliases `features.stream_ticket_header=true` and `features.stream_query_ticket=false`. Protocol version `1.0` alone is insufficient; there is no credential-URL fallback.

Build 20261111.008 synchronizes the specifications, provenance and documentation only, and lets the Kit subset check accept the new player root `/episodes` (with one new check test); Rust source and Cargo inputs are unchanged and the SDK stays 0.1.0-rc.2. On a workstation, 22 Rust tests and strict Clippy passed, as did the specification, typed-contract, credential and link checks; designated-server checks, real-player acceptance and publication have not run. The original designated-server evidence for Build 20261029.004 (source `c8ee971b6660418bd4cb2836be0286fab76d29b6`: 21 Rust tests and a 1 MiB mock CLI flow) belongs to that source only.

Start with the [quickstart](docs/quickstart.md), [header rules](docs/credentials-in-headers.md), [playback guide](docs/playback.md), [Rust SDK](sdks/rust/README.md), [curl example](examples/curl/README.md), or [Rust CLI](examples/rust-cli/README.md). The [versioning guide](docs/versioning.md) describes compatibility and acceptance status.

[Connection guidance](docs/connecting.md), [discovery guidance](docs/discovery.md) and [client request headers](docs/client-request-headers.md) define trusted origins, local-HTTP opt-in, ports, metadata, per-install UUIDs, purpose and tracing. UDP client networking is not implemented here; deployment and real-LAN acceptance remain separate.

The Rust candidate models login, browsing, versions, prepared playback sessions, ticket renewal, a bounded byte range, progress and explicit session termination. Video requests use `X-SXAPI-Stream-Ticket`; subtitles additionally retain the original login Bearer. Streamix Server currently serves the original file without transcoding. Players must declare their actual supported formats.

TypeScript, Swift and Kotlin SDKs are unimplemented future work. No packages for those languages are advertised or published.

This public repository contains client contracts and integration source only. Specifications are synchronized from the server-generated full and client specifications and must not be edited here.

[Commercial proprietary license](LICENSE): publicly readable source and authorized SDK/API integration, not an open-source license. SDK binary redistribution requires separate permission. Calling the HTTP API does not require SDK redistribution. Third-party dependency licenses remain applicable; see [NOTICES](NOTICES.md).

Tracked in [GitHub #1](https://github.com/fxxkrlab/streamix_kit/issues/1) and [FXX-566](https://linear.app/fxxkrlab/issue/FXX-566). Source availability does not imply compilation, integration, publication or successful playback acceptance.

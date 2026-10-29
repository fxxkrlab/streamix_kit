# StreamixKit

SXAPI v1 integration documents and an asynchronous Rust SDK for player authors.

**0.1.0-rc.1 · Build 20261029.001 — source candidate, not released or server-validated.**

The minimum compatible server release and source are not bound yet; see [SOURCE.json](spec/SOURCE.json). This candidate requires header-only tickets. It never falls back to credential URLs. Discovery must report `features.playback_ticket_header=true` and `features.credentials_in_urls=false`; protocol version `1.0` alone is insufficient.

Start with the [quickstart](docs/quickstart.md), [header rules](docs/credentials-in-headers.md), [playback guide](docs/playback.md), [Rust SDK](sdks/rust/README.md), [curl example](examples/curl/README.md), or [Rust CLI](examples/rust-cli/README.md). The [versioning guide](docs/versioning.md) describes compatibility and acceptance status.

[Connection guidance](docs/connecting.md), [discovery guidance](docs/discovery.md) and [client request headers](docs/client-request-headers.md) define trusted origins, local-HTTP opt-in, ports, metadata, per-install UUIDs, purpose and tracing. UDP client networking is not implemented here; server changes await final RC.9 verification.

The Rust candidate models login, browsing, versions, prepared playback sessions, ticket renewal, a bounded byte range, progress and explicit session termination. Video requests use `X-SXAPI-Stream-Ticket`; subtitles additionally retain the original login Bearer. Streamix Server currently serves the original file without transcoding. Players must declare their actual supported formats.

TypeScript, Swift and Kotlin SDKs are unimplemented future work. No packages for those languages are advertised or published.

This public repository contains client contracts and integration source only. Specifications are synchronized from the server-generated full and client specifications and must not be edited here.

[Commercial proprietary license](LICENSE): publicly readable source and authorized SDK/API integration, not an open-source license. SDK binary redistribution requires separate permission. Calling the HTTP API does not require SDK redistribution. Third-party dependency licenses remain applicable; see [NOTICES](NOTICES.md).

Tracked in [GitHub #1](https://github.com/fxxkrlab/streamix_kit/issues/1) and [FXX-566](https://linear.app/fxxkrlab/issue/FXX-566). Source availability does not imply compilation, integration, publication or successful playback acceptance.

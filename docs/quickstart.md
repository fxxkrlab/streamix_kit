# Five-minute integration path

This source candidate is bound to the unreleased 0.9.0-rc.9 server contract. [SOURCE.json](../spec/SOURCE.json) records exact provenance; designated-server SDK validation and integration acceptance remain pending.

1. Choose the trusted **origin**, normally HTTPS such as `https://streamix.example`. Trusted-local HTTP is an explicit opt-in, not a TLS fallback; see [connecting](connecting.md). API paths begin with `/sxapi/v1`. Resolve returned root-relative video paths against the origin, not the API prefix.
2. Read `GET /sxapi/v1/capabilities`. Require `features.playback_ticket_header=true`, `features.credentials_in_urls=false` and the relevant feature such as `playback=true`. Do not infer support from protocol version alone.
3. Create a session with `POST /sxapi/v1/sessions` and JSON `username`/`password`. Keep `access_token` private and send it only as `Authorization: Bearer` on authenticated API calls. The login JSON is the unavoidable credential exchange; never put it in an address or log it.
4. Read `/libraries`, then page `/media?library_id=…&limit=20`. Follow `next_cursor`; do not fetch an entire catalog for the home screen. For a movie, choose a real version from `/media/{id}/versions`. For television choose an actual episode first.
5. Prepare `POST /playback/sessions` with the media/episode/version IDs and a truthful `profile`. It returns a session plus a short ticket. This grants an original-file attempt; it does not prove decoding.
6. Resolve the returned `stream_path` against the trusted origin and send the ticket only in `X-SXAPI-Stream-Ticket`. A single bounded `Range: bytes=0-1048575` tests the first 1 MiB. Require 206 and validate Content-Range before treating it as a fragment.
7. Before new requests near ticket expiry, use `POST /playback/sessions/{id}/tickets`. A renewed ticket cannot revive an expired login or ended session. Send sequential progress reports based on actual playback, then `DELETE /playback/sessions/{id}` and revoke `/sessions/current` when done.

See the [curl example](../examples/curl/README.md) for a bounded command-line flow and [Rust CLI](../examples/rust-cli/README.md) for the typed SDK flow. Never enable shell tracing, HTTP verbose dumps or logging of request/response bodies during these flows.
